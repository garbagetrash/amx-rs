use amx::{Index4, Normal, X8, XBytes, XRow, YBytes, YRow, prelude::*};
use either::{Left, Right};
use quickcheck::TestResult;

fn overlaps(x: std::ops::Range<usize>, y: std::ops::Range<usize>) -> bool {
    x.start < y.end && x.end > y.start
}

#[quickcheck_macros::quickcheck]
fn qc_genlut_lut8x16(
    table_row: usize,
    index_offset: usize,
    indices_in_y: bool,
    out_row: usize,
    mut indices: Vec<u8>,
    mut values: Vec<u8>,
) -> TestResult {
    values.resize_with(64, u8::default);
    indices.resize_with(32, u8::default);
    let out_row = out_row % 8;
    let table_row = table_row % 8;
    let index_offset = index_offset % 512;
    if overlaps(
        index_offset..index_offset + 64,
        table_row * 64..table_row * 64 + 64,
    ) || overlaps(
        index_offset..index_offset + 64,
        table_row * 64 + 512..table_row * 64 + 64 + 512,
    ) {
        return TestResult::discard();
    }

    let mut got = [0u8; 64];
    let mut ctx = amx::AmxCtx::new().unwrap();
    unsafe {
        indices.resize_with(64, u8::default);

        // Load `indices` at byte offset `index_offset`
        let mut index_row_1 = [0u8; 64];
        let mut index_row_2 = [0u8; 64];
        let sub = index_offset % 64;
        index_row_1[sub..].copy_from_slice(&indices[..64 - sub]);
        index_row_2[..sub].copy_from_slice(&indices[64 - sub..]);
        if indices_in_y {
            ctx.load512(index_row_1.as_ptr(), YRow(index_offset / 64));
            ctx.load512(index_row_2.as_ptr(), YRow(index_offset / 64 + 1));
        } else {
            ctx.load512(index_row_1.as_ptr(), XRow(index_offset / 64));
            ctx.load512(index_row_2.as_ptr(), XRow(index_offset / 64 + 1));
        }

        // Load `values` at the row `table_row`
        ctx.load512(values.as_ptr(), XRow(table_row));
    }

    // Perform table lookup
    ctx.lut(
        if indices_in_y {
            Left(YBytes(index_offset))
        } else {
            Right(XBytes(index_offset))
        },
        XRow(table_row),
        XRow(out_row),
        (Normal, Index4, X8),
    );

    // Read the result
    unsafe { ctx.store512(got.as_mut_ptr(), XRow(out_row)) };
    let all_x = unsafe { std::mem::transmute::<_, [[u64; 8]; 8]>(ctx.read_x()) };

    let expected: Vec<u8> = (0..64)
        .map(|i| {
            let idx = (indices[i / 2] >> (i % 2 * 4)) as usize & 0xf;
            values[idx]
        })
        .collect();

    assert_eq!(
        got[..],
        expected[..],
        "got = {:?}, expected = {:?}",
        got,
        expected,
    );

    TestResult::passed()
}
