//! Text and text iterator tests

use crate::memory::{initialize_test_memory, reset_test_memory};

use motoko_rts::memory::Memory;
use motoko_rts::text::{
    blob_of_text, decode_code_point, text_compare, text_concat, text_len, text_of_str,
    text_singleton, text_size,
};
use motoko_rts::text_iter::{text_iter, text_iter_done, text_iter_next};
use motoko_rts::types::{Bytes, TAG_BLOB_T, Value};

use std::convert::TryFrom;

static STR: &str = "abcdefgh";

struct TextIter<'a, M: Memory> {
    obj: Value,
    mem: &'a mut M,
}

impl<'a, M: Memory> TextIter<'a, M> {
    fn from_text(mem: &'a mut M, text: Value) -> Self {
        TextIter {
            obj: unsafe { text_iter(mem, text) },
            mem,
        }
    }
}

impl<'a, M: Memory> Iterator for TextIter<'a, M> {
    type Item = char;

    fn next(&mut self) -> Option<char> {
        unsafe {
            if text_iter_done(self.obj) == 1 {
                None
            } else {
                let next = text_iter_next(self.mem, self.obj);
                Some(char::try_from(next).unwrap())
            }
        }
    }
}

pub unsafe fn test() {
    println!("Testing text and text iterators ...");

    let mut mem = initialize_test_memory();

    println!("  Testing decode_code_point and text_singleton for ASCII");
    for i in 0..=255u32 {
        let char = char::try_from(i).unwrap();
        let mut str = String::new();
        str.push(char);
        let mut out: usize = 0;
        let char_decoded = decode_code_point(str.as_ptr(), &mut out as *mut _);
        assert_eq!(out, str.len());
        assert_eq!(char::try_from(char_decoded).unwrap(), char);

        let text = text_singleton(&mut mem, char as u32);
        assert_eq!(TextIter::from_text(&mut mem, text).collect::<String>(), str);
    }

    println!("  Testing text blob iteration");
    for i in 0..8 {
        let str = &STR[0..i + 1];
        let text = text_of_str(&mut mem, str);
        assert_eq!(text.tag(), TAG_BLOB_T);
        let iter = TextIter::from_text(&mut mem, text);
        assert_eq!(iter.collect::<String>(), str);
    }

    println!("  Testing concatenation");
    concat1(&mut mem);

    let mut long_text = String::from("");
    for i in 0..=255u32 {
        long_text.push(char::try_from(i).unwrap());
    }
    concat_test(&mut mem, vec![String::from("")]);
    concat_test(&mut mem, vec![String::from(""), String::from("")]);
    concat_test(&mut mem, vec![String::from("a"), String::from("")]);
    concat_test(&mut mem, vec![String::from(""), String::from("b")]);
    concat_test(&mut mem, vec![String::from("a"), String::from("b")]);
    concat_test(
        &mut mem,
        vec![String::from(""), String::from(""), String::from("")],
    );
    concat_test(
        &mut mem,
        vec![
            String::from("test-"),
            String::from("abc"),
            String::from("-0123"),
            String::from("_!?"),
            String::from("äöü"),
            long_text,
        ],
    );

    drop(mem);

    reset_test_memory();
}

unsafe fn concat1<M: Memory>(mem: &mut M) {
    // A simple test extracted from a QuickCheck generated test case
    let strs = ["a", "öabcdef", "y"];

    let mut obj = text_of_str(mem, "");
    for str in &strs {
        let str_obj = text_of_str(mem, str);
        obj = text_concat(mem, obj, str_obj);
    }

    let expected = strs.concat();

    // Check number of characters
    assert_eq!(text_len(obj), expected.chars().count());

    // Check text size in bytes
    assert_eq!(text_size(obj), Bytes(expected.len()));

    // Generate blob
    let text_blob = blob_of_text(mem, obj);

    // Check number of characters in blob
    assert_eq!(text_len(text_blob), expected.chars().count());

    // Check blob size in bytes
    assert_eq!(text_size(text_blob), Bytes(expected.len()));

    // Check blob iteration
    let blob = blob_of_text(mem, obj);
    assert_eq!(TextIter::from_text(mem, blob).collect::<String>(), expected);

    // Check blob-concat comparison
    assert_eq!(text_compare(text_blob, obj), 0);

    // Check concat iteration
    assert_eq!(TextIter::from_text(mem, obj).collect::<String>(), expected);
}

fn concat_test<M: Memory>(mem: &mut M, strs: Vec<String>) {
    unsafe {
        let mut obj = text_of_str(mem, "");
        for str in &strs {
            let str_obj = text_of_str(mem, str);
            obj = text_concat(mem, obj, str_obj);
        }

        let expected = strs.concat();

        // Check number of characters
        assert_eq!(text_len(obj), expected.chars().count());

        // Check text size in bytes
        assert_eq!(text_size(obj), Bytes(expected.len()));

        // Generate blob
        let text_blob = blob_of_text(mem, obj);

        // Check number of characters in blob
        assert_eq!(text_len(text_blob), expected.chars().count());

        // Check blob size in bytes
        assert_eq!(text_size(text_blob), Bytes(expected.len()));

        // Check blob iteration
        let blob = blob_of_text(mem, obj);
        assert_eq!(TextIter::from_text(mem, blob).collect::<String>(), expected);

        // Check blob-concat comparison
        assert_eq!(text_compare(text_blob, obj), 0);

        // Check concat iteration
        assert_eq!(TextIter::from_text(mem, obj).collect::<String>(), expected);
    }
}
