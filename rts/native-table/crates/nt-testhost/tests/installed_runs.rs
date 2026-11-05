//! A run installed in table memory as an image of 64 KiB blocks, read through the image's block entries,
//! answers every lookup exactly as the same bytes read from memory do.

use native_table::area::new_root;
use native_table::image::{Image, ImageReader};
use native_table::index::{Run, RunParams};
use native_table::key::enc_bytes;
use native_table::key::probe::{KeyProbe, Kind};
use native_table::tree::find;
use native_table::ImageId;
use nt_testhost::runs::{build, VecItems};
use nt_testhost::{VecHost, VecSink, XorShift, FILLS};

#[test]
fn installed_runs_read_like_memory_runs() {
    let mut r = XorShift(31);
    let mut items = Vec::new();
    for i in 0..60_000u64 {
        let t: Vec<u8> = (0..1 + r.below(30)).map(|_| b'a' + r.below(4) as u8).collect();
        let mut s = VecSink::default();
        enc_bytes(&mut s, &t).unwrap();
        items.push((s.0, i, false));
    }
    let items = VecItems::sorted(items);
    let p = RunParams {
        index: 3,
        relative: false,
        w0: 0,
        key_run_l: 0,
        table_uid: 7,
        schema_hash: 8,
        key_spec_hash: 9,
        force: None,
    };
    for fill in FILLS {
        let built = build(&items, &p, fill, false).unwrap();
        assert!(built.image.len() > 3 * 65_536, "the run spans several image blocks");
        let mem = built.open();
        // Install the same bytes in table memory, appended in 4 KiB pieces.
        let mut host = VecHost::new(fill);
        let tbl = new_root(&mut host).unwrap();
        let mut img = Image::EMPTY;
        for piece in built.image.chunks(4_096) {
            img.append(&mut host, tbl, piece).unwrap();
        }
        let id = ImageId::Run { index: 3, start_seq: 1 };
        let reader = ImageReader::new(&host, id, img);
        let inst = Run::open(reader, 0, 0, u64::MAX).unwrap();
        let (a, b) = (mem.main().unwrap(), inst.main().unwrap());
        assert_eq!((a.shape(), a.root()), (b.shape(), b.root()));
        for i in (0..items.0.len()).step_by(37) {
            let (k, rid, _) = &items.0[i];
            for kind in [Kind::Low, Kind::Row(*rid), Kind::Count] {
                let probe = KeyProbe { key: k, kind };
                assert_eq!(
                    find(a.shape(), &a, a.root(), &probe).unwrap(),
                    find(b.shape(), &b, b.root(), &probe).unwrap()
                );
            }
        }
        // A read past the image's end faults with the run's id.
        assert!(matches!(
            native_table::index::RunImage::read(&inst_reader(&host, img), built.image.len() as u64, 8),
            Err(native_table::Fault::Corrupt(c)) if c.image == id
        ));
    }
}

fn inst_reader(host: &VecHost, img: Image) -> ImageReader<'_, VecHost> {
    ImageReader::new(host, ImageId::Run { index: 3, start_seq: 1 }, img)
}
