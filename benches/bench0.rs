
#![feature(test)]


extern crate test;

use test::bench::*;

#[bench]
fn bench0(b: &mut Bencher) {
    let argb: &mut [u8] = &mut [0; 256 * 256 * 4];

    b.iter(|| {
        for i in 0..argb.len() {
            argb[i] = argb[i] / 2;
        }
    })
}


#[bench]
fn bench1(b: &mut Bencher) {
    use gradient::bitmap::*;
    let argb: &mut [u8] = &mut [0; 256 * 256 * 4];

    let mut btmp: BitmapRef<ArgbU32Layout<3, 2, 1, 0>> = BitmapRef::from_bytes(argb, 256, 256).unwrap();

    b.iter(|| {
        btmp.iter_mut().for_each(|mut x| {
            *x.a_u8() = *x.a_u8() / 2;
            *x.r_u8() = *x.r_u8() / 2;
            *x.g_u8() = *x.g_u8() / 2;
            *x.b_u8() = *x.b_u8() / 2;
        });
    })
}
