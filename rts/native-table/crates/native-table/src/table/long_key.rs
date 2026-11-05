//! A secondary index key too long for a stack buffer: a count that hands the key over again from its cells for
//! each comparison.

use core::cell::Cell;

use crate::fault::Fault;
use crate::index::secondary::Secondary;
use crate::key::probe::KeyFeed;
use crate::key::Sink;
use crate::mem::PageHost;

use super::indexes::{enc_source, Decl};
use super::layout::Shape;
use super::RowSource;

/// The key the cells of `src` give index `d`, longer than `KEY_STACK`, handed over by encoding it again from the
/// cells each time, with no buffer.
struct SrcKey<'a, H: ?Sized, R: ?Sized> {
    shape: &'a Shape,
    h: &'a H,
    d: &'a Decl,
    src: Cell<Option<&'a mut R>>,
}

impl<H: PageHost + ?Sized, R: RowSource + ?Sized> KeyFeed for SrcKey<'_, H, R> {
    fn feed<S: Sink>(&self, s: &mut S) -> Result<(), Fault> {
        let src = self.src.take().ok_or(Fault::BadRef)?;
        let r = enc_source(self.shape, self.h, self.d, &mut *src, s);
        self.src.set(Some(src));
        r
    }
}

/// `count` for a key longer than `KEY_STACK`: the stored keys of the index's tree and of each uploaded segment
/// compared with the key as its cells encode it again, piece by piece.
#[inline(never)]
pub(super) fn count_long<H: PageHost + ?Sized, K: RowSource + ?Sized>(
    h: &H,
    tbl: u64,
    shape: &Shape,
    d: &Decl,
    key: &mut K,
) -> Result<i128, Fault> {
    let k = SrcKey { shape, h, d, src: Cell::new(Some(key)) };
    Secondary::read(h, tbl, d.id)?.count_fed(h, &k)
}
