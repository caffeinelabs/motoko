#![allow(dead_code)]

use motoko_rts_macros::classical_persistence;
use motoko_rts_macros::enhanced_orthogonal_persistence;

use crate::print::*;
use crate::types::*;

use core::fmt::Write;

/// Print an object. The argument can be a skewed pointer to a boxed object, or a tagged scalar.
#[cfg(feature = "ic")]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn print_value(value: Value) {
    let mut buf = [0u8; 1000];
    let mut write_buf = WriteBuf::new(&mut buf);

    match value.get() {
        PtrOrScalar::Scalar(scalar) => print_tagged_scalar(&mut write_buf, scalar),
        PtrOrScalar::Ptr(ptr) => print_boxed_object(&mut write_buf, ptr),
    }

    print(&write_buf);
}

pub unsafe fn dump_heap(
    heap_base: usize,
    hp: usize,
    static_root_location: *mut Value,
    continuation_table_location: *mut Value,
) {
    print_continuation_table(continuation_table_location);
    print_static_roots(*static_root_location);
    print_heap(heap_base, hp);
}

pub(crate) unsafe fn print_continuation_table(continuation_tbl_loc: *mut Value) {
    if !crate::continuation_table::table_initialized() {
        println!(100, "Continuation table not initialized");
        return;
    }

    let arr = (*continuation_tbl_loc).as_array();
    let len = (*arr).len;

    if len == 0 {
        println!(50, "Continuation table empty");
        return;
    }

    println!(50, "Continuation table: {}", len);

    let mut buf = [0u8; 1000];
    let mut write_buf = WriteBuf::new(&mut buf);

    for i in 0..len {
        let elem = arr.get(i);
        if is_valid_pointer(elem) {
            let _ = write!(&mut write_buf, "{}: ", i);
            print_boxed_object(&mut write_buf, elem.get_ptr());
            print(&write_buf);
            write_buf.reset();
        }
    }
    println!(50, "End of continuation table");
}

#[classical_persistence]
fn is_valid_pointer(value: Value) -> bool {
    value.is_ptr()
}

#[enhanced_orthogonal_persistence]
fn is_valid_pointer(value: Value) -> bool {
    value.is_non_null_ptr()
}

pub(crate) unsafe fn print_static_roots(static_roots: Value) {
    let static_roots = static_roots.as_array();
    println!(100, "static roots at {:#x}", static_roots as usize);

    let len = (*static_roots).len;

    if len == 0 {
        println!(50, "Static roots empty");
        return;
    }

    println!(50, "Static roots: {}", len);

    let mut buf = [0u8; 1000];
    let mut write_buf = WriteBuf::new(&mut buf);

    let payload_addr = static_roots.payload_addr();
    for i in 0..len {
        let field_addr = payload_addr.add(i);
        let _ = write!(&mut write_buf, "{}: {:#x} --> ", i, field_addr as usize);
        print_boxed_object(&mut write_buf, (*field_addr).get_ptr());
        print(&write_buf);
        write_buf.reset();
    }

    println!(50, "End of static roots");
}

unsafe fn print_heap(heap_start: usize, heap_end: usize) {
    println!(
        200,
        "Heap start={:#x}, heap end={:#x}, size={} bytes",
        heap_start,
        heap_end,
        heap_end - heap_start
    );

    let mut buf = [0u8; 1000];
    let mut write_buf = WriteBuf::new(&mut buf);

    let mut p = heap_start;
    let mut i: Words<usize> = Words(0);
    while p < heap_end {
        print_boxed_object(&mut write_buf, p);
        print(&write_buf);
        write_buf.reset();

        let obj_size = block_size(p);
        p += obj_size.to_bytes().as_usize();
        i += obj_size;
    }
}

unsafe fn print_tagged_scalar(buf: &mut WriteBuf, p: usize) {
    let _ = write!(buf, "<Scalar {:#x}>", p);
}

/// Print boxed object at address `p`.
pub(crate) unsafe fn print_boxed_object(buf: &mut WriteBuf, p: usize) {
    let _ = write!(buf, "{:#x}: ", p);

    let forward = (*(p as *mut Value)).forward();
    if forward.get_ptr() != p {
        let _ = write!(buf, "<forwarded to {:#x}>", forward.get_ptr());
        return;
    }

    let obj = p as *mut Obj;
    let tag = obj.tag();

    if tag == 0 {
        return;
    }

    match tag {
        TAG_OBJECT => {
            let object = obj as *mut Object;
            let _ = write!(
                buf,
                "<Object size={:#x} hash_ptr={:#x} field=[",
                object.size(),
                get_obj_hash_pointer(object),
            );
            for i in 0..object.size() {
                let val = object.get(i);
                let _ = write!(buf, "{:#x}", val.get_raw());

                if let PtrOrScalar::Ptr(indirectee_ptr) = val.get() {
                    let _ = write!(buf, " (indirectee=");
                    print_boxed_object(buf, indirectee_ptr);
                    let _ = write!(buf, ")");
                }

                if i != object.size() - 1 {
                    let _ = write!(buf, ",");
                }
            }
            let _ = write!(buf, "]>");
        }
        TAG_ARRAY_I | TAG_ARRAY_M | TAG_ARRAY_T | TAG_ARRAY_S | TAG_ARRAY_SLICE_MIN.. => {
            let array = obj as *mut Array;
            let _ = write!(buf, "<Array len={:#x}", (*array).len);

            for i in 0..::core::cmp::min(10, (*array).len) {
                let _ = write!(buf, " {:#x}", array.get(i).get_raw());
            }

            if (*array).len > 10 {
                let _ = write!(buf, " …>");
            } else {
                let _ = write!(buf, ">");
            }
        }
        TAG_BITS64_U | TAG_BITS64_S | TAG_BITS64_F => {
            let bits64 = obj as *const Bits64;
            let _ = write!(buf, "<Bits64 {:#x}>", (*bits64).bits());
        }
        TAG_MUTBOX => {
            let mutbox = obj as *const MutBox;
            let _ = write!(buf, "<MutBox field={:#x}>", (*mutbox).field.get_raw());
        }
        TAG_CLOSURE => {
            let closure = obj as *const Closure;
            let _ = write!(buf, "<Closure size={:#x}>", (*closure).size);
        }
        TAG_SOME => {
            let some = obj as *const Some;
            let _ = write!(buf, "<Some field={:#x}>", (*some).field.get_raw());
        }
        TAG_VARIANT => {
            let variant = obj as *const Variant;
            let _ = write!(
                buf,
                "<Variant tag={:#x} field={:#x}>",
                (*variant).tag,
                (*variant).field.get_raw()
            );
        }
        TAG_BLOB_B | TAG_BLOB_T | TAG_BLOB_P | TAG_BLOB_A => {
            let blob = obj.as_blob();
            let _ = write!(buf, "<Blob len={:#x}>", blob.len().as_usize());
        }
        TAG_FWD_PTR => {
            let ind = obj as *const FwdPtr;
            let _ = write!(buf, "<Forwarding to {:#x}>", (*ind).fwd.get_raw());
        }
        TAG_BIGINT => {
            // Add more details here as needed
            let _ = write!(buf, "<BigInt>");
        }
        TAG_CONCAT => {
            let concat = obj.as_concat();
            let _ = write!(
                buf,
                "<Concat n_bytes={:#x} obj1={:#x} obj2={:#x}>",
                (*concat).n_bytes.as_usize(),
                (*concat).text1.get_raw(),
                (*concat).text2.get_raw()
            );
        }
        TAG_ONE_WORD_FILLER => {
            let _ = write!(buf, "<One word filler>",);
        }
        TAG_FREE_SPACE => {
            let free_space = obj as *const FreeSpace;
            let _ = write!(buf, "<Free space {} words>", (*free_space).words.as_usize());
        }
        other => {
            let _ = write!(buf, "<??? {} ???>", other);
        }
    }
}

#[classical_persistence]
unsafe fn get_obj_hash_pointer(object: *mut Object) -> usize {
    (*object).hash_ptr
}

#[enhanced_orthogonal_persistence]
unsafe fn get_obj_hash_pointer(object: *mut Object) -> usize {
    (*object).hash_blob.get_raw()
}
