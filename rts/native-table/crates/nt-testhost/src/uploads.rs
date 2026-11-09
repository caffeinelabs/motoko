//! Uploaded segments for tests: a table with one secondary index and its run list in a `VecHost`, and runs
//! built by the run builder, installed in table memory and listed in the run list's loaded-run list.

use native_table::area::new_root;
use native_table::consts::PAGE;
use native_table::image::{Image, ImageReader};
use native_table::index::directory::take_run_seq;
use native_table::index::run::F_KEY_RUNS;
use native_table::index::runlist::{
    image_radix, RunDesc, RunList, D_BELIEVED, D_COUNT_TREE, D_FILTER, D_KEY_RUNS, D_RELATIVE,
};
use native_table::index::secondary::Secondary;
use native_table::mem::{PageHost, Region};
use native_table::ImageId;
use nt_builder::RunParams;

use crate::runs::{build, VecItems};
use crate::VecHost;

/// The index id of the test table's index.
pub const IX: u8 = 1;

/// A table with one secondary index (id 1, W 8) and its run list.
pub struct TTable {
    pub h: VecHost,
    pub tbl: u64,
    pub ix: u8,
    /// The run list block.
    pub run_list: u64,
}

impl TTable {
    /// A fresh table: root block, the index's descriptor in the index directory, an empty run list.
    pub fn new(fill: u8) -> TTable {
        let mut h = VecHost::new(fill);
        let tbl = new_root(&mut h).unwrap();
        // The root's fields outside the area records, as a table's creation writes them.
        {
            let s = h.get_mut(Region::Block(tbl)).unwrap();
            s[..160].fill(0);
            s[928..].fill(0);
            s[8..16].copy_from_slice(&0x1111u64.to_le_bytes());
            s[16..24].copy_from_slice(&0x2222u64.to_le_bytes());
            s[96..104].copy_from_slice(&1u64.to_le_bytes());
        }
        Secondary::declare(&mut h, tbl, IX, 8, 0x3333).unwrap();
        let run_list = RunList::create(&mut h, tbl).unwrap().block;
        TTable { h, tbl, ix: IX, run_list }
    }

    /// The run list as it is stored.
    pub fn list(&self) -> RunList {
        RunList::load(&self.h, self.run_list).unwrap()
    }

    /// Every listed run, in listing order.
    pub fn runs(&self) -> Vec<RunDesc> {
        let rl = self.list();
        (0..rl.loaded.runs).map(|i| rl.loaded_get(&self.h, i).unwrap()).collect()
    }

    fn params(&self, relative: bool, force: Option<(u64, u64)>, key_run_l: u8) -> RunParams {
        RunParams {
            index: self.ix,
            relative,
            w0: 0,
            key_run_l,
            table_uid: 0x1111,
            schema_hash: 0x2222,
            key_spec_hash: 0x3333,
            force,
        }
    }

    /// Builds a run of `items` (a forced (W, V) when given, key runs of field length `key_run_l` when not 0)
    /// with absolute rids and copies it into table memory; returns its descriptor (not listed yet).
    pub fn make_run(&mut self, items: &VecItems, force: Option<(u64, u64)>, key_run_l: u8) -> RunDesc {
        let built = build(items, &self.params(false, force, key_run_l), self.h.fill_pattern(), false).unwrap();
        let img = self.install_image(&built.image);
        let h = built.plan.header;
        let mut flags = 0u8;
        if h.has_filter() {
            flags |= D_FILTER;
        }
        if h.flags & F_KEY_RUNS != 0 {
            flags |= D_KEY_RUNS;
        }
        if h.has_count_tree() {
            flags |= D_COUNT_TREE;
        }
        self.desc_of(&img, h.rows + h.tombs, (0, h.rid_lo, h.rid_hi), flags, (h.height, h.v, h.w))
    }

    /// The descriptor of an installed image: its items, (rid base, smallest rid, largest rid), flags and the
    /// run's (tree height, V, W); a new start sequence.
    fn desc_of(&mut self, img: &Image, items: u64, rids: (u64, u64, u64), flags: u8, geo: (u64, u64, u64)) -> RunDesc {
        let r = image_radix(img).unwrap();
        RunDesc {
            root: r.root,
            items,
            rid_base: rids.0,
            rid_lo: rids.1,
            rid_hi: rids.2,
            start_seq: take_run_seq(&mut self.h, self.tbl).unwrap(),
            img_height: r.height,
            flags,
            tree_height: geo.0 as u8,
            v: geo.1 as u16,
            w: geo.2 as u16,
        }
    }

    /// Copies image bytes into table memory, in 4 KiB pieces; returns the image in reference form.
    pub fn install_image(&mut self, bytes: &[u8]) -> Image {
        let mut img = Image::EMPTY;
        for piece in bytes.chunks(4_096) {
            img.append(&mut self.h, self.tbl, piece).unwrap();
        }
        img.ref_form(&mut self.h, self.tbl).unwrap()
    }

    /// Lists a run in the loaded-run list.
    pub fn list_run(&mut self, d: RunDesc) {
        let mut rl = self.list();
        rl.loaded_append(&mut self.h, self.tbl, &d).unwrap();
        rl.store(&mut self.h).unwrap();
    }

    /// Builds and lists a loaded run as an upload brings it: rids relative to `base`, over a reservation of
    /// `groups` groups, believed.
    pub fn add_loaded(&mut self, items: &VecItems, base: u64, groups: u64) -> RunDesc {
        let built = build(items, &self.params(true, None, 0), self.h.fill_pattern(), false).unwrap();
        let mut img = self.install_image(&built.image);
        img.believed = true;
        let h = built.plan.header;
        let mut flags = D_BELIEVED | D_RELATIVE;
        if h.has_filter() {
            flags |= D_FILTER;
        }
        if h.has_count_tree() {
            flags |= D_COUNT_TREE;
        }
        let rids = (base, base, base + groups * 4_096 - 1);
        let d = self.desc_of(&img, h.rows + h.tombs, rids, flags, (h.height, h.v, h.w));
        self.list_run(d);
        d
    }

    /// The bytes of a run's image.
    pub fn image_bytes(&self, d: &RunDesc) -> Vec<u8> {
        let img = d.image(&self.h).unwrap();
        let r = ImageReader::new(&self.h, ImageId::Runtime, img);
        let mut out = Vec::with_capacity(img.length as usize);
        let mut at = 0;
        while at < img.length {
            let n = (PAGE - at % PAGE).min(img.length - at);
            out.extend_from_slice(r.bytes(at, n).unwrap());
            at += n;
        }
        out
    }
}
