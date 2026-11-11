//! Long values: images of kind 5 in area 15. {common header 32 B (kind 5); content bytes u64 @32; content
//! u8 @40 (1 a row image, 2 a value, 3 a patch value); flags u8 @41; zero u16 @42; leaf ordinal u32 @44;
//! zero u64 @48; zero u64 @56}, then the content from offset 64, zero-padded to a
//! multiple of 8.
//!
//! A long value has exactly one referent: a slot's long stub, a row image's long item, or an entry of a
//! group's long-value list. Every move of its 24-byte image reference zeroes the source.

use crate::consts::BLOCK;
use crate::fault::{rt, Corruption, Fault, ImageId};
use crate::image::{Image, ImageReader};
use crate::mem::{copy_bytes, put_u16, put_u32, put_u64, put_u8, PageHost};
use crate::schema::put_common_header;
use crate::value::Utf8Stream;

use super::RowSource;

/// Bytes before the content.
pub const HEAD: u64 = 64;
/// Image kind of a long value.
pub const KIND_LONG: u8 = 5;
/// Content kinds.
pub const CONTENT_ROW: u8 = 1;
pub const CONTENT_VALUE: u8 = 2;

/// The image bytes of a long value of `len` content bytes.
pub fn image_bytes(len: u64) -> Result<u64, Fault> {
    let n = HEAD.checked_add(len).and_then(|x| x.checked_add(7)).ok_or(rt(Corruption::Range, len))?;
    Ok(n & !7)
}

/// What a long value records about itself besides its content.
#[derive(Clone, Copy, Debug)]
pub struct Meta {
    pub uid: u64,
    pub schema_hash: u64,
    pub content: u8,
    pub leaf: u32,
}

/// A long value being written: its header first, then its content in order, staged through a 4 KiB buffer
/// so each block of the image is written once.
pub struct LongOut {
    pub img: Image,
    tbl: u64,
    stage: [u8; 4_096],
    staged: usize,
    /// Content bytes still to come.
    left: u64,
    /// Content bytes the host copied straight from the argument.
    pub filled: u64,
}

impl LongOut {
    /// Starts a long value of `len` content bytes.
    pub fn begin<H: PageHost>(h: &mut H, tbl: u64, meta: &Meta, len: u64) -> Result<LongOut, Fault> {
        let total = image_bytes(len)?;
        let mut head = [0u8; 64];
        put_common_header(&mut head, KIND_LONG, 0, total, meta.uid, meta.schema_hash)?;
        put_u64(&mut head, 32, len)?;
        put_u8(&mut head, 40, meta.content)?;
        put_u8(&mut head, 41, 0)?;
        put_u16(&mut head, 42, 0)?;
        put_u32(&mut head, 44, meta.leaf)?;
        put_u64(&mut head, 48, 0)?;
        put_u64(&mut head, 56, 0)?;
        let mut o = LongOut { img: Image::EMPTY, tbl, stage: [0; 4_096], staged: 0, left: len, filled: 0 };
        o.stage_bytes(h, &head)?;
        o.left = len;
        Ok(o)
    }

    fn stage_bytes<H: PageHost>(&mut self, h: &mut H, b: &[u8]) -> Result<(), Fault> {
        let mut rest = b;
        while !rest.is_empty() {
            let room = self.stage.len().saturating_sub(self.staged);
            let n = room.min(rest.len());
            let (now, later) = rest.split_at_checked(n).ok_or(rt(Corruption::Range, 0))?;
            let end = self.staged.saturating_add(n);
            copy_bytes(self.stage.get_mut(self.staged..end).ok_or(rt(Corruption::Range, 0))?, now);
            self.staged = end;
            rest = later;
            if self.staged == self.stage.len() {
                self.flush(h)?;
            }
        }
        Ok(())
    }

    fn flush<H: PageHost>(&mut self, h: &mut H) -> Result<(), Fault> {
        if self.staged > 0 {
            let s = self.stage.get(..self.staged).ok_or(rt(Corruption::Range, 0))?;
            self.img.append(h, self.tbl, s)?;
            self.staged = 0;
        }
        Ok(())
    }

    pub fn put<H: PageHost>(&mut self, h: &mut H, b: &[u8]) -> Result<(), Fault> {
        let n = b.len() as u64;
        self.left = self.left.checked_sub(n).ok_or(rt(Corruption::Range, n))?;
        self.stage_bytes(h, b)
    }

    /// Appends `len` content bytes that the host copies straight from the message's argument into the
    /// image's pages through `src.fill(leaf, ·)`, validated as UTF-8 across pieces when `text`.
    pub fn fill<H: PageHost, R: RowSource + ?Sized>(
        &mut self,
        h: &mut H,
        src: &mut R,
        leaf: u32,
        len: u64,
        text: bool,
    ) -> Result<(), Fault> {
        self.left = self.left.checked_sub(len).ok_or(rt(Corruption::Range, len))?;
        // Bring the image to a 4 KiB boundary or flush what is staged, so the pieces go to the pages.
        self.flush(h)?;
        let mut utf = Utf8Stream::new();
        let mut copied = 0u64;
        self.img.append_with(h, self.tbl, len, |at, dst| {
            src.fill(leaf, at, dst)?;
            copied = copied.saturating_add(dst.len() as u64);
            if text && !utf.feed(dst) {
                return Err(Fault::BadValue { leaf });
            }
            Ok(())
        })?;
        if text && !utf.finish() {
            return Err(Fault::BadValue { leaf });
        }
        self.filled = self.filled.saturating_add(copied);
        Ok(())
    }

    /// Ends the value: the padding to 8 and the last staged bytes. Returns the image.
    pub fn finish<H: PageHost>(mut self, h: &mut H) -> Result<Image, Fault> {
        if self.left != 0 {
            return Err(rt(Corruption::Count, self.left));
        }
        let pad = self.img.length.wrapping_add(self.staged as u64).wrapping_neg() & 7;
        let zeros = [0u8; 8];
        self.stage_bytes(h, zeros.get(..usize::try_from(pad).unwrap_or(0)).unwrap_or(&[]))?;
        self.flush(h)?;
        // A long value is named by a 24-byte reference: 2 to 8 blocks take a radix block of their own.
        self.img.ref_form(h, self.tbl)
    }
}

/// A long value of content 2 holding `bytes`.
pub fn from_bytes<H: PageHost>(h: &mut H, tbl: u64, meta: &Meta, bytes: &[u8]) -> Result<Image, Fault> {
    let mut o = LongOut::begin(h, tbl, meta, bytes.len() as u64)?;
    o.put(h, bytes)?;
    o.finish(h)
}

/// A long value of content 2 holding a value still in the message's argument; returns the image and the
/// bytes the host copied.
pub fn from_arg<H: PageHost, R: RowSource + ?Sized>(
    h: &mut H,
    tbl: u64,
    meta: &Meta,
    src: &mut R,
    leaf_id: u32,
    len: u64,
    text: bool,
) -> Result<(Image, u64), Fault> {
    let mut o = LongOut::begin(h, tbl, meta, len)?;
    o.fill(h, src, leaf_id, len, text)?;
    let n = o.filled;
    Ok((o.finish(h)?, n))
}

pub fn content_len<H: PageHost + ?Sized>(h: &H, img: Image) -> Result<u64, Fault> {
    ImageReader::new(h, ImageId::Runtime, &img).u64_at(32)
}

/// Copies content bytes [at, at + out.len()) of a long value into `out`, across image blocks.
pub fn read<H: PageHost + ?Sized>(h: &H, img: Image, at: u64, out: &mut [u8]) -> Result<(), Fault> {
    let r = ImageReader::new(h, ImageId::Runtime, &img);
    read_image(&r, HEAD.checked_add(at).ok_or(rt(Corruption::Range, at))?, out)
}

/// Copies image bytes [off, off + out.len()) through a reader, in pieces that never cross an image block.
pub fn read_image<H: PageHost + ?Sized>(r: &ImageReader<'_, '_, H>, off: u64, out: &mut [u8]) -> Result<(), Fault> {
    let mut at = off;
    let mut rest = out;
    while !rest.is_empty() {
        let room = crate::consts::PAGE.wrapping_sub(at % crate::consts::PAGE);
        let n = usize::try_from(room).unwrap_or(usize::MAX).min(rest.len());
        let (now, later) = rest.split_at_mut_checked(n).ok_or(rt(Corruption::Range, at))?;
        copy_bytes(now, r.bytes(at, n as u64)?);
        at = at.checked_add(n as u64).ok_or(rt(Corruption::Range, at))?;
        rest = later;
    }
    Ok(())
}

/// Overwrites content bytes [at, at + src.len()) of a long value (to zero a reference that moved out).
pub fn write_at<H: PageHost>(h: &mut H, img: Image, at: u64, src: &[u8]) -> Result<(), Fault> {
    let mut off = HEAD.checked_add(at).ok_or(rt(Corruption::Range, at))?;
    let mut rest = src;
    while !rest.is_empty() {
        let k = off / crate::consts::PAGE;
        let o = off % crate::consts::PAGE;
        let p = img.entry(h, k)?;
        if p == 0 {
            return Err(rt(Corruption::Link, k));
        }
        let room = BLOCK.wrapping_sub(o % BLOCK);
        let n = usize::try_from(room).unwrap_or(usize::MAX).min(rest.len());
        let (now, later) = rest.split_at_checked(n).ok_or(rt(Corruption::Range, off))?;
        // o < 65,536, so the block index is below 16.
        let j = u32::try_from(o / BLOCK).map_err(|_| rt(Corruption::Range, o))?;
        let b = h.block_of(p, j);
        crate::mem::put(h.get_mut(crate::mem::Region::Block(b))?, o % BLOCK, now)?;
        off = off.wrapping_add(n as u64);
        rest = later;
    }
    Ok(())
}

/// Zero bytes for padding writes.
pub const ZEROS: [u8; 4_096] = [0; 4_096];
