use amx::{XBytes, YBytes, XRow, YRow, ZRow, prelude::*};


fn main() {
    let mut ctx = amx::AmxCtx::new().unwrap();

    for i in 0..8 {
        // Get together 16 f32s (1 register of 64 bytes)
        let x: [f32; 16] = (16*i+1..16*(i+1)+1).map(|x| x as f32).collect::<Vec<_>>().try_into().unwrap();

        // Load x into registers X, Y
        unsafe { ctx.load512(x.as_ptr(), XRow(i)) };
        unsafe { ctx.load512(x.as_ptr(), YRow(i)) };
    }

    ctx.outer_product_f32_xy_to_z(Some(XBytes(0)), Some(YBytes(0)), ZRow(0), false);
    ctx.outer_product_f32_xy_to_z(Some(XBytes(196)), Some(YBytes(196)), ZRow(1), false);
    ctx.outer_product_f32_xy_to_z(Some(XBytes(128)), Some(YBytes(128)), ZRow(2), false);
    ctx.outer_product_f32_xy_to_z(Some(XBytes(64)), Some(YBytes(64)), ZRow(3), false);

    let xbytes = ctx.read_x();
    let rx: Vec<f32> = xbytes.as_slice().chunks(4)
        .map(|y| {
            let yy: [u8; 4] = y.try_into().unwrap();
            f32::from_le_bytes(yy)
        }).collect();

    let ybytes = ctx.read_y();
    let ry: Vec<f32> = ybytes.as_slice().chunks(4)
        .map(|y| {
            let yy: [u8; 4] = y.try_into().unwrap();
            f32::from_le_bytes(yy)
        }).collect();

    for (i, (x, y)) in rx.iter().zip(ry.iter()).enumerate() {
        println!("rx[{}]: {}, ry[{}]: {}", i, x, i, y);
    }

    let zbytes = ctx.read_z();
    let rz: Vec<f32> = zbytes.as_slice().chunks(4)
        .map(|y| {
            let yy: [u8; 4] = y.try_into().unwrap();
            f32::from_le_bytes(yy)
        }).collect();

    for (i, z) in rz.iter().enumerate() {
        println!("rz[{}]: {}", i, z);
    }
}
