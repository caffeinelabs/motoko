//! What a table needs from its schema image, derived once when the image is written and kept in the table
//! root, which every open reads: each leaf's scalar type, the row image's layout, the hot state's layout with
//! its running records, the primary key's spec, and where the schema's index declarations lie.
//!
//! Leaves are read one at a time from the schema page, so no operation needs memory proportional to the
//! number of leaves: every pass over a row walks the leaves in ordinal order.

use crate::consts::{FORMAT_TABLE, HOT_MAX, W_MAX, W_MIN};
use crate::fault::{corrupt, Corruption, Fault, ImageId};
use crate::hash::{nt_hash64, Hasher64};
use crate::mem::{put, put_u64, rd_u16, rd_u32, rd_u64, rd_u8, sub, Owner, PageHost, Region};
use crate::schema::{node, Leaf, SchemaView, LEAF_BYTES, LEAF_KEY, LEAF_NULLABLE};
use crate::stats::running::{RKind, LIST_AT};

use super::group::align8;
use super::root;
use crate::value::ty;

fn rt(kind: Corruption, at: u64) -> Fault {
    corrupt(ImageId::Runtime, kind, at)
}

/// The scalar type of a flat leaf.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ty {
    Bool,
    /// An unsigned integer of 1, 2, 4 or 8 bytes.
    U(u8),
    /// A signed integer of 1, 2, 4 or 8 bytes.
    I(u8),
    Nat,
    Int,
    F32,
    F64,
    Char,
    Text,
    Blob,
    Principal,
    U128,
    I128,
    /// `[u8; N]`.
    Fixed(u16),
    Unit,
}

impl Ty {
    /// The type of a leaf record, or `None` for a kind or type code the table does not hold.
    pub fn of(l: &Leaf) -> Option<Ty> {
        match l.kind {
            node::UNIT => Some(Ty::Unit),
            node::SCALAR => Some(match l.code {
                ty::BOOL => Ty::Bool,
                ty::U8 => Ty::U(1),
                ty::U16 => Ty::U(2),
                ty::U32 => Ty::U(4),
                ty::U64 => Ty::U(8),
                ty::I8 => Ty::I(1),
                ty::I16 => Ty::I(2),
                ty::I32 => Ty::I(4),
                ty::I64 => Ty::I(8),
                ty::NAT => Ty::Nat,
                ty::INT => Ty::Int,
                ty::F32 => Ty::F32,
                ty::F64 => Ty::F64,
                ty::CHAR => Ty::Char,
                ty::TEXT => Ty::Text,
                ty::BLOB => Ty::Blob,
                ty::PRINCIPAL => Ty::Principal,
                ty::U128 => Ty::U128,
                ty::I128 => Ty::I128,
                ty::BYTES => Ty::Fixed(l.width),
                _ => return None,
            }),
            _ => None,
        }
    }

    /// Bytes of the leaf's cell in a row image's fixed region.
    pub fn fixed_width(self) -> u64 {
        match self {
            Ty::Bool => 1,
            Ty::U(w) | Ty::I(w) => u64::from(w),
            Ty::Nat | Ty::Int | Ty::U128 | Ty::I128 => 16,
            Ty::F32 | Ty::Char => 4,
            Ty::F64 => 8,
            Ty::Principal => 30,
            Ty::Fixed(n) => u64::from(n),
            Ty::Text | Ty::Blob | Ty::Unit => 0,
        }
    }

    /// Whether the leaf has an entry in a row image's item table.
    pub fn variable(self) -> bool {
        matches!(self, Ty::Text | Ty::Blob | Ty::Nat | Ty::Int)
    }

    /// The running record an integer leaf gets, holding its total; `None` for any other leaf.
    pub fn running(self) -> Option<RKind> {
        match self {
            Ty::U(_) | Ty::I(_) => Some(RKind::Int),
            Ty::Nat | Ty::Int => Some(RKind::Num),
            Ty::U128 | Ty::I128 => Some(RKind::Wide),
            _ => None,
        }
    }
}

/// One leaf, as the table uses it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LeafInfo {
    pub ordinal: u16,
    pub id: u32,
    pub ty: Ty,
    pub nullable: bool,
    pub key: bool,
    /// The leaf's type expression in the type pool.
    pub path: u32,
    /// The schema versions that added and dropped it (0: live).
    pub added: u32,
    pub dropped: u32,
}

impl LeafInfo {
    /// Whether rows of schema version v hold the leaf.
    pub fn at(&self, v: u32) -> bool {
        self.added <= v && (self.dropped == 0 || self.dropped > v)
    }

    /// Whether the leaf is live in the table's schema.
    pub fn live(&self) -> bool {
        self.dropped == 0
    }
}

/// The layout of a row image of one schema version: its leaves R, the bytes of each of its two bitmaps,
/// the fixed region's offset and bytes, and its variable leaves V.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RowHead {
    pub version: u32,
    pub r: u64,
    pub bitmap: u64,
    pub fixed_at: u64,
    pub fixed: u64,
    pub vars: u64,
}

impl RowHead {
    /// The layout of a row image of version `version` from its leaves R, fixed bytes and variable leaves.
    pub fn of(version: u32, r: u64, fixed: u64, vars: u64) -> RowHead {
        let bitmap = r.div_ceil(8);
        // bitmap ≤ 8,192.
        let fixed_at = align8(8u64.wrapping_add(bitmap.wrapping_mul(2)));
        RowHead { version, r, bitmap, fixed_at, fixed, vars }
    }

    /// Bytes from a row image's start to its item table.
    pub fn items_table_at(&self) -> u64 {
        self.fixed_at.wrapping_add(self.fixed)
    }

    /// Bytes from a row image's start to its first item.
    pub fn items_at(&self) -> u64 {
        self.items_table_at().wrapping_add(self.vars.wrapping_mul(4))
    }
}

/// What a walk over the leaf list needs of one leaf, read in place from its record: the schema versions that
/// added and dropped it, its cell's bytes in a row image's fixed region and whether it has an item.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Span {
    pub added: u32,
    pub dropped: u32,
    pub width: u64,
    pub variable: bool,
}

impl Span {
    /// Whether rows of schema version v hold the leaf.
    pub fn at(&self, v: u32) -> bool {
        self.added <= v && (self.dropped == 0 || self.dropped > v)
    }
}

/// The table's view of its schema image, kept by `Table`: derived from the image when it is written, and stored
/// in the table root (`store`), where every open reads it (`stored`).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Shape {
    /// The schema image's page.
    pub page: u64,
    pub hash: u64,
    pub version: u32,
    /// Leaf ordinals L (all row-level in a flat table), dropped leaves included.
    pub leaves: u64,
    /// The layout of a row image of the schema version.
    pub head: RowHead,
    leaves_at: u64,
    /// Bytes of each of a row image's two bitmaps.
    pub bitmap: u64,
    /// The fixed region's offset in a row image.
    pub fixed_at: u64,
    /// The fixed region's bytes.
    pub fixed: u64,
    /// Variable leaves V.
    pub vars: u64,
    /// Running records R and the bytes of their leaf list and of the records.
    pub run_r: u64,
    pub run_list: u64,
    pub run_bytes: u64,
    /// Bytes of the integer leaves' totals that have no room in the hot state: the records the totals space
    /// holds after its header (`totals`).
    pub tot_bytes: u64,
    /// H: the first row byte of every buffer block.
    pub hot_h: u64,
    /// Key leaves, the primary index's W₀ and its key spec hash.
    pub key_leaves: u64,
    pub key_w: u64,
    pub key_hash: u64,
    /// Whether every leaf is a key leaf (a set).
    pub set: bool,
    /// The schema image's index declarations: their offset in the schema page and their number, the primary
    /// ones included (`indexes`).
    pub index_at: u64,
    pub index_decls: u64,
    /// The latest schema version that dropped a leaf (0: none): the column release's generation.
    pub dropped: u32,
    /// The columns the schema marks dropped: what `cleaned` counts once the release has run.
    pub dropped_columns: u32,
}

/// The index kind byte of the primary index in its key spec hash.
pub const PRIMARY_KIND: u8 = 1;

/// Bytes of the shape a table root holds at `root::SHAPE` (`Shape::store`): 24 words and their `nt_hash64`.
pub const STORED_BYTES: u64 = STORED as u64;
const STORED: usize = 200;

impl Shape {
    /// The shape of the schema image held at the start of page `page`. A leaf a flat table cannot hold (a
    /// variant, List, Union, Present, Enc or Table leaf), or a dropped leaf, is `Corrupt(Runtime, Leaf)`.
    pub fn open<H: PageHost + ?Sized>(h: &H, page: u64) -> Result<Shape, Fault> {
        let bytes = h.get(Region::Page(page))?;
        let v = SchemaView::open(bytes)?;
        let mut s = Shape {
            page,
            hash: v.hash,
            version: v.version,
            leaves: u64::from(v.leaves),
            head: RowHead::default(),
            leaves_at: u64::from(v.layout.leaves),
            bitmap: 0,
            fixed_at: 0,
            fixed: 0,
            vars: 0,
            run_r: 0,
            run_list: 0,
            run_bytes: 0,
            tot_bytes: 0,
            hot_h: 0,
            key_leaves: 0,
            key_w: 0,
            key_hash: 0,
            set: true,
            index_at: u64::from(v.layout.indexes),
            index_decls: u64::from(v.indexes),
            dropped: 0,
            dropped_columns: 0,
        };
        for i in 0..v.columns {
            if v.column(i)?.flags & 1 != 0 {
                s.dropped_columns = s.dropped_columns.wrapping_add(1);
            }
        }
        if s.leaves == 0 {
            return Err(rt(Corruption::Leaf, 0));
        }
        let mut r = 0u64;
        let mut key_width = 0u64;
        for o in 0..s.leaves {
            let l = s.leaf(h, o)?;
            s.dropped = s.dropped.max(l.dropped);
            if !l.live() {
                continue;
            }
            r = r.wrapping_add(1);
            // Widths are at most 65,535 per leaf and leaves at most 65,535: the sums stay far below 2^63.
            s.fixed = s.fixed.wrapping_add(l.ty.fixed_width());
            if l.ty.variable() {
                s.vars = s.vars.wrapping_add(1);
            }
            if l.key {
                s.key_leaves = s.key_leaves.wrapping_add(1);
                key_width = key_width.wrapping_add(key_kind(l.ty).default_width()).wrapping_add(u64::from(l.nullable));
            } else {
                s.set = false;
            }
        }
        if s.key_leaves == 0 {
            return Err(rt(Corruption::Leaf, 0));
        }
        s.bitmap = r.div_ceil(8);
        // bitmap ≤ 8,192.
        s.fixed_at = align8(8u64.wrapping_add(s.bitmap.wrapping_mul(2)));
        s.head =
            RowHead { version: s.version, r, bitmap: s.bitmap, fixed_at: s.fixed_at, fixed: s.fixed, vars: s.vars };
        s.key_w = key_width.div_ceil(8).wrapping_mul(8).clamp(W_MIN, W_MAX);
        // Rule of room: the fixed part first, then each integer leaf's running record with its list entry in
        // ordinal order while it fits. A total that does not fit goes to the totals space.
        let mut r = 0u64;
        let mut recs = 0u64;
        for o in 0..s.leaves {
            let l = s.leaf(h, o)?;
            let Some(k) = l.ty.running().filter(|_| l.live()) else { continue };
            let b = k.bytes();
            let list = align8(r.wrapping_add(1).wrapping_mul(2));
            if LIST_AT.wrapping_add(list).wrapping_add(recs).wrapping_add(b) <= HOT_MAX {
                r = r.wrapping_add(1);
                recs = recs.wrapping_add(b);
            } else {
                // At most 65,535 leaves of 32 bytes.
                s.tot_bytes = s.tot_bytes.wrapping_add(b);
            }
        }
        s.run_r = r;
        s.run_list = align8(r.wrapping_mul(2));
        s.run_bytes = recs;
        s.hot_h = LIST_AT.wrapping_add(s.run_list).wrapping_add(recs);
        s.key_hash = s.key_spec_hash(h)?;
        Ok(s)
    }

    /// The shape's words as the table root holds them: `FORMAT_TABLE`, the schema page, schema hash and
    /// schema version the shape was derived from, then its other fields.
    fn words(&self) -> [u64; 24] {
        [
            u64::from(FORMAT_TABLE),
            self.page,
            self.hash,
            u64::from(self.version),
            self.leaves,
            self.head.r,
            self.leaves_at,
            self.bitmap,
            self.fixed_at,
            self.fixed,
            self.vars,
            self.run_r,
            self.run_list,
            self.run_bytes,
            self.tot_bytes,
            self.hot_h,
            self.key_leaves,
            self.key_w,
            self.key_hash,
            u64::from(self.set),
            self.index_at,
            self.index_decls,
            u64::from(self.dropped),
            u64::from(self.dropped_columns),
        ]
    }

    /// Writes the shape into table root `tbl` at `root::SHAPE`: its words, then their `nt_hash64`. Whatever
    /// writes a table's schema image stores the image's shape with it: `Table::create` and every schema change.
    pub fn store<H: PageHost>(&self, h: &mut H, tbl: u64) -> Result<(), Fault> {
        let mut b = [0u8; STORED];
        let mut at = 0u64;
        for w in self.words() {
            put_u64(&mut b, at, w)?;
            at = at.wrapping_add(8);
        }
        let check = nt_hash64(sub(&b, 0, at)?);
        put_u64(&mut b, at, check)?;
        put(h.get_mut(Region::Block(tbl), Owner(0))?, root::SHAPE, &b)
    }

    /// The shape table root `tbl` holds, when it is the shape of the schema image the root names: written by
    /// this table format, and derived from the root's schema page, schema hash and schema version. `None` for
    /// any other: none stored, one of another format, or one of an earlier schema (every schema change writes
    /// its image to a new page, also one that keeps the version). Its hash is not checked: a shape whose other
    /// words changed is taken as it is.
    pub fn stored<H: PageHost + ?Sized>(h: &H, tbl: u64) -> Result<Option<Shape>, Fault> {
        let r = h.get(Region::Block(tbl))?;
        let b = sub(r, root::SHAPE, STORED_BYTES)?;
        // The 24 words in `words`' order, six at a time.
        let mut w = [[0u64; 6]; 4];
        let mut at = 0u64;
        for x in w.iter_mut().flatten() {
            *x = rd_u64(b, at)?;
            at = at.wrapping_add(8);
        }
        let [w0, w1, w2, w3] = w;
        let [format, page, hash, version, leaves, rows_r] = w0;
        let [leaves_at, bitmap, fixed_at, fixed, vars, run_r] = w1;
        let [run_list, run_bytes, tot_bytes, hot_h, key_leaves, key_w] = w2;
        let [key_hash, set, index_at, index_decls, dropped, dropped_columns] = w3;
        let (Ok(version), Ok(dropped), Ok(dropped_columns)) =
            (u32::try_from(version), u32::try_from(dropped), u32::try_from(dropped_columns))
        else {
            return Ok(None);
        };
        if format != u64::from(FORMAT_TABLE)
            || page != rd_u64(r, root::SCHEMA_PAGE)?
            || hash != rd_u64(r, root::SCHEMA_HASH)?
            || version != rd_u32(r, root::SCHEMA_VERSION)?
        {
            return Ok(None);
        }
        Ok(Some(Shape {
            page,
            hash,
            version,
            leaves,
            head: RowHead { version, r: rows_r, bitmap, fixed_at, fixed, vars },
            leaves_at,
            bitmap,
            fixed_at,
            fixed,
            vars,
            run_r,
            run_list,
            run_bytes,
            tot_bytes,
            hot_h,
            key_leaves,
            key_w,
            key_hash,
            set: set != 0,
            index_at,
            index_decls,
            dropped,
            dropped_columns,
        }))
    }

    /// Leaf `o`.
    pub fn leaf<H: PageHost + ?Sized>(&self, h: &H, o: u64) -> Result<LeafInfo, Fault> {
        if o >= self.leaves {
            return Err(rt(Corruption::Leaf, o));
        }
        let b = h.get(Region::Page(self.page))?;
        let l = Leaf::read(b, self.leaves_at.wrapping_add(o.wrapping_mul(LEAF_BYTES)))?;
        let ty = Ty::of(&l).ok_or(rt(Corruption::Leaf, o))?;
        // o < leaves ≤ 65,535.
        #[allow(clippy::cast_possible_truncation)]
        let ordinal = o as u16;
        Ok(LeafInfo {
            ordinal,
            id: l.id,
            ty,
            nullable: l.flags & LEAF_NULLABLE != 0,
            key: l.flags & LEAF_KEY != 0,
            path: l.path,
            added: l.added,
            dropped: l.dropped,
        })
    }

    /// Leaf o's span, read in place.
    pub fn span_of<H: PageHost + ?Sized>(&self, h: &H, o: u64) -> Result<Span, Fault> {
        let mut out = None;
        self.spans(h, o, o.saturating_add(1), |_, s| {
            out = Some(s);
            Ok(false)
        })?;
        out.ok_or(rt(Corruption::Leaf, o))
    }

    /// Hands the spans of leaves [from, to) (clamped to the leaves) to `f` in ordinal order, each read in place
    /// from its leaf record, until `f` returns `false`: a walk over many leaves pays a few loads per leaf, not
    /// `leaf`'s whole record.
    pub fn spans<H, F>(&self, h: &H, from: u64, to: u64, mut f: F) -> Result<(), Fault>
    where
        H: PageHost + ?Sized,
        F: FnMut(u64, Span) -> Result<bool, Fault>,
    {
        let b = h.get(Region::Page(self.page))?;
        let mut o = from;
        let end = to.min(self.leaves);
        while o < end {
            let at = self.leaves_at.wrapping_add(o.wrapping_mul(LEAF_BYTES));
            let l = Leaf {
                kind: rd_u8(b, at.wrapping_add(4))?,
                code: rd_u8(b, at.wrapping_add(5))?,
                width: rd_u16(b, at.wrapping_add(8))?,
                ..Leaf::default()
            };
            let t = Ty::of(&l).ok_or(rt(Corruption::Leaf, o))?;
            let s = Span {
                added: rd_u32(b, at.wrapping_add(16))?,
                dropped: rd_u32(b, at.wrapping_add(20))?,
                width: t.fixed_width(),
                variable: t.variable(),
            };
            if !f(o, s)? {
                break;
            }
            // o < end ≤ 65,535.
            o = o.wrapping_add(1);
        }
        Ok(())
    }

    /// The layout of a row image of schema version v: the table's own when v is its version, else computed
    /// over the leaves version v holds, O(L).
    pub fn head_for<H: PageHost + ?Sized>(&self, h: &H, v: u32) -> Result<RowHead, Fault> {
        if v == self.version {
            return Ok(self.head);
        }
        let mut hd = RowHead { version: v, ..RowHead::default() };
        for o in 0..self.leaves {
            let l = self.leaf(h, o)?;
            if !l.at(v) {
                continue;
            }
            hd.r = hd.r.wrapping_add(1);
            hd.fixed = hd.fixed.wrapping_add(l.ty.fixed_width());
            if l.ty.variable() {
                hd.vars = hd.vars.wrapping_add(1);
            }
        }
        hd.bitmap = hd.r.div_ceil(8);
        hd.fixed_at = align8(8u64.wrapping_add(hd.bitmap.wrapping_mul(2)));
        Ok(hd)
    }

    /// Leaf l's offsets in a row image of version v: (fixed offset, variable index, bitmap bit), or `None`
    /// when rows of version v lack the leaf. O(ordinal).
    pub fn offsets_at<H: PageHost + ?Sized>(
        &self,
        h: &H,
        v: u32,
        l: &LeafInfo,
    ) -> Result<Option<(u64, u64, u64)>, Fault> {
        if !l.at(v) {
            return Ok(None);
        }
        let hd = self.head_for(h, v)?;
        let (mut fixed, mut var, mut bit) = (hd.fixed_at, 0u64, 0u64);
        for o in 0..u64::from(l.ordinal) {
            let k = self.leaf(h, o)?;
            if !k.at(v) {
                continue;
            }
            fixed = fixed.wrapping_add(k.ty.fixed_width());
            if k.ty.variable() {
                var = var.wrapping_add(1);
            }
            bit = bit.wrapping_add(1);
        }
        Ok(Some((fixed, var, bit)))
    }

    /// The ordinal of the live leaf with id `id`.
    pub fn ordinal_of<H: PageHost + ?Sized>(&self, h: &H, id: u32) -> Result<Option<u64>, Fault> {
        let b = h.get(Region::Page(self.page))?;
        for o in 0..self.leaves {
            let at = self.leaves_at.wrapping_add(o.wrapping_mul(LEAF_BYTES));
            if rd_u32(b, at)? == id && rd_u32(b, at.wrapping_add(20))? == 0 {
                return Ok(Some(o));
            }
        }
        Ok(None)
    }

    /// The key spec hash of the primary index: `nt_hash64` over {index kind u8; `FORMAT_KEY` u16; parts u16;
    /// per key leaf in ordinal order: leaf id u32, flags u8 (bit 0 nullable)}.
    fn key_spec_hash<H: PageHost + ?Sized>(&self, h: &H) -> Result<u64, Fault> {
        let n = self.key_leaves;
        let mut hs = Hasher64::new(5u64.wrapping_add(n.wrapping_mul(5)));
        hs.update(&[PRIMARY_KIND]);
        hs.update(&crate::consts::FORMAT_KEY.to_le_bytes());
        hs.update(&u16::try_from(n).unwrap_or(u16::MAX).to_le_bytes());
        for o in 0..self.leaves {
            let l = self.leaf(h, o)?;
            if l.key && l.live() {
                hs.update(&l.id.to_le_bytes());
                hs.update(&[u8::from(l.nullable)]);
            }
        }
        Ok(hs.finish())
    }

    /// Bytes from a row image's start to its item table.
    pub fn items_table_at(&self) -> u64 {
        self.head.items_table_at()
    }

    /// Bytes from a row image's start to its first item.
    pub fn items_at(&self) -> u64 {
        self.head.items_at()
    }
}

/// The key kind a key leaf encodes as.
pub fn key_kind(t: Ty) -> crate::key::KeyKind {
    use crate::key::KeyKind as K;
    match t {
        Ty::Bool => K::Bool,
        Ty::U(1) => K::U8,
        Ty::U(2) => K::U16,
        Ty::U(4) => K::U32,
        Ty::U(_) => K::U64,
        Ty::I(1) => K::I8,
        Ty::I(2) => K::I16,
        Ty::I(4) => K::I32,
        Ty::I(_) => K::I64,
        Ty::Nat => K::Nat,
        Ty::Int => K::Int,
        Ty::F32 => K::F32,
        Ty::F64 => K::F64,
        Ty::Char => K::Char,
        Ty::Text => K::Text,
        Ty::Blob => K::Blob,
        Ty::Principal => K::Principal,
        Ty::U128 => K::U128,
        Ty::I128 => K::I128,
        Ty::Fixed(n) => K::Fixed(n),
        Ty::Unit => K::Unit,
    }
}
