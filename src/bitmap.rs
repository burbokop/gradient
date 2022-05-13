use std::{ops::{Sub, Mul}, marker::PhantomData};

use byte_slice_cast::*;

pub trait PixLayout {
    type Pixel;

    fn a_u8(p: &mut Self::Pixel) -> &mut u8;
    fn r_u8(p: &mut Self::Pixel) -> &mut u8;
    fn g_u8(p: &mut Self::Pixel) -> &mut u8;
    fn b_u8(p: &mut Self::Pixel) -> &mut u8;

    fn get_argb_u32(p: &Self::Pixel) -> u32;
    fn set_argb_u32(p: &mut Self::Pixel, argb: u32);

}

#[derive(Debug)]
pub struct ArgbU32Layout<const A: usize, const R: usize, const G: usize, const B: usize> {}

impl<const A: usize, const R: usize, const G: usize, const B: usize> PixLayout for ArgbU32Layout<A, R, G, B> {
    type Pixel = [u8; 4];

    #[inline]
    fn a_u8(p: &mut Self::Pixel) -> &mut u8 { &mut p[A] }
    #[inline]
    fn r_u8(p: &mut Self::Pixel) -> &mut u8 { &mut p[R] }
    #[inline]
    fn g_u8(p: &mut Self::Pixel) -> &mut u8 { &mut p[G] }
    #[inline]
    fn b_u8(p: &mut Self::Pixel) -> &mut u8 { &mut p[B] }

    #[inline]
    fn get_argb_u32(p: &Self::Pixel) -> u32 {
        from_argb([p[A], p[R], p[G], p[B]])
    }

    #[inline]
    fn set_argb_u32(p: &mut Self::Pixel, argb: u32) {
        let pp = to_argb(argb);

        p[A] = pp[0];
        p[R] = pp[1];
        p[G] = pp[2];
        p[B] = pp[3];
    }

}

#[derive(Debug)]
pub struct RgbU24Layout<const R: usize, const G: usize, const B: usize> {}

impl<const R: usize, const G: usize, const B: usize> PixLayout for RgbU24Layout<R, G, B> {
    type Pixel = [u8; 3];

    #[inline]
    fn a_u8(p: &mut Self::Pixel) -> &mut u8 {
        &mut p[R]
    }

    #[inline]
    fn r_u8(p: &mut Self::Pixel) -> &mut u8 { &mut p[R] }

    #[inline]
    fn g_u8(p: &mut Self::Pixel) -> &mut u8 { &mut p[G] }

    #[inline]
    fn b_u8(p: &mut Self::Pixel) -> &mut u8 { &mut p[B] }

    #[inline]
    fn get_argb_u32(p: &Self::Pixel) -> u32 {
        from_argb([0xff, p[R], p[G], p[B]])
    }

    #[inline]
    fn set_argb_u32(p: &mut Self::Pixel, argb: u32) {
        let pp = to_argb(argb);

        p[R] = pp[1];
        p[G] = pp[2];
        p[B] = pp[3];
    }
}


pub struct PixRef<'p, L: PixLayout> {
    data: &'p mut L::Pixel,
}

impl<'p, L: PixLayout> PixRef<'p, L> {
    #[inline]
    pub fn a_u8(&mut self) -> &mut u8 { L::a_u8(self.data) }
    #[inline]
    pub fn r_u8(&mut self) -> &mut u8 { L::r_u8(self.data) }
    #[inline]
    pub fn g_u8(&mut self) -> &mut u8 { L::g_u8(self.data) }
    #[inline]
    pub fn b_u8(&mut self) -> &mut u8 { L::b_u8(self.data) }

    #[inline]
    pub fn get_argb_u32(&self) -> u32 { L::get_argb_u32(self.data) }
    #[inline]
    pub fn set_argb_u32(&mut self, argb: u32) { L::set_argb_u32(self.data, argb) }
}

#[derive(Debug)]
pub struct BitmapRef<'p, L: PixLayout> {
    data: &'p mut [L::Pixel],
    width: usize,
    height: usize,
    l: PhantomData<L> 
}

#[inline]
pub fn to_argb(p: u32) -> [u8; 4] {
    let p_be = p.to_be();
    [ 
        (p_be >> 24) as u8,
        (p_be >> 16) as u8,
        (p_be >> 8) as u8,
        (p_be >> 0) as u8,
    ]
}

#[inline]
pub fn from_argb(argb: [u8; 4]) -> u32 {
    u32::from_be(((argb[0] as u32) << 24)
    | ((argb[1] as u32) << 16)
    | ((argb[2] as u32) << 8)
    | ((argb[3] as u32) << 0))
}

impl<'p, L: PixLayout> BitmapRef<'p, L> {
    pub fn new(
        data: &'p mut [L::Pixel],
        width: usize,
        height: usize
    ) -> Self {
        Self { data: data, width: width, height: height, l: Default::default() }
    }
    
    pub fn from_bytes(
        data: &'p mut [u8],
        width: usize,
        height: usize,
    ) -> Option<Self> 
    where L::Pixel: FromByteSlice
    {
        let depth = std::mem::size_of::<L::Pixel>();
        if width * height * depth <= data.len() {
            match data.as_mut_slice_of() {
                Ok(data) => Some(Self { 
                    data: data,
                    width: width, 
                    height: height,
                    l: Default::default()
                }),
                Err(_) => None,
            }
        } else { None }
    }

    //pub fn to_bytes(self) -> &'p [u8] 
    //where P: ToByteSlice
    //{
    //    self.data.as_byte_slice()
    //}

    /// consider using to_be after called pixel
    pub fn pixel(&mut self, x: usize, y: usize) -> PixRef<'_, L> {
        PixRef { data: &mut self.data[x + y * self.width] }
    }

    /// consider using to_be after called pixel or from_be if you writing pixel
    //pub fn pixel_mut(&mut self, x: usize, y: usize) -> &mut PixRef<'p, L> {
    //    &mut self.data[x + y * self.width]
    //}

    pub fn width(&self) -> usize { self.width }
    pub fn height(&self) -> usize { self.height }

    pub fn iter_mut(&mut self) -> impl Iterator<Item=PixRef<'_, L>> {
        self.data.iter_mut().map(|p| PixRef { data: p })
    }

    //pub fn sub_by<F: Fn(P, P) -> P>(&self, other: &Self, result: &mut Self, f: F) -> bool 
    //where P: Copy
    //{
    //    if self.width == other.width && self.width == result.width && self.height == other.height && self.height == result.height {
    //        self
    //            .data
    //            .iter()
    //            .zip(other.data.iter())
    //            .zip(result.data.iter_mut())
    //            .for_each(|((a, b), r)| *r = f(*a, *b));
//
    //        true
    //    } else { false }
    //}

    //pub fn sub(&self, other: &Self, result: &mut Self) -> bool 
    //where P: Sub<Output=P> + Copy
    //{
    //    self.sub_by(other, result, |a, b| a - b)
    //}

    //pub fn zip_mut<F: Fn(&mut P, &P)>(&mut self, other: &Self) -> impl Iterator<Item=(&'s mut P, &'o P)> + 's + 'o {
    //    self
    //        .data
    //        .iter_mut()
    //        .zip(other.data.iter()).for_each(f)
    //}

    pub fn clone_by<'a, 'b, F: Fn(PixRef<'b, L>, PixRef<'a, L>)>(&'a mut self, result: &'b mut Self, f: F) -> bool {
        if self.width == result.width && self.height == result.height {
            self.iter_mut()
                .zip(result.iter_mut())
                .for_each(|(a, r)| f(r, a));

            true
        } else { false }
    }


    //pub fn mul_by<F: Fn(P, P) -> P>(&self, multiplier: P, result: &mut Self, f: F) -> bool 
    //where P: Copy
    //{
    //    self.clone_by(result, |r, a| *r = f(*a, multiplier))
    //}

    //pub fn mul(&self, multiplier: P, result: &mut Self) -> bool 
    //where P: Mul<Output=P> + Copy
    //{
    //    self.mul_by(multiplier, result, |a, b| a * b)
    //}

    pub fn for_each_mut<F: Fn(PixRef<'_, L>)>(&mut self, f: F) {
        self.iter_mut().for_each(f);
    }

    //pub fn mul_self(&mut self, multiplier: P) 
    //where P: Mul<Output=P> + Copy
    //{
    //    self.for_each_mut(|r| *r = *r * multiplier)
    //}

}

//impl<'p, P: 'static> Sub for Bitmap<'p, P> {
//    type Output = Bitmap<'static, P>;
//
//    fn sub(self, rhs: Self) -> Self::Output {
//        Bitmap
//
//
//        todo!()
//    }
//}

#[cfg(test)]
mod tests {
    use crate::bitmap::ArgbU32Layout;

    use super::BitmapRef;

    #[test]
    fn test_argb() {
        let mut data: &mut [u8] = &mut [0xff, 0x88, 0x44, 0x22, 0x1, 0x2, 0x3, 0x4];

        let mut btmp: BitmapRef<ArgbU32Layout<3, 2, 1, 0>> = BitmapRef::from_bytes(&mut data, 2, 1).unwrap();

        println!("btmp: {:?}", btmp);

        assert_eq!(btmp.width(), 2);
        assert_eq!(btmp.height(), 1);
        assert_eq!(btmp.pixel(0, 0).get_argb_u32(), 0xff884422_u32);

        btmp.pixel(0, 0).set_argb_u32(0xff224400);// = u32::from_be(0xff224400);

        //let res_data: &[u8] = btmp.to_bytes();
        //assert_eq!(res_data.len(), 4);
        //assert_eq!(res_data, &[0xff, 0x22, 0x44, 0x88]);
        assert_eq!(data, &[0xff, 0x22, 0x44, 0x00, 0x1, 0x2, 0x3, 0x4]);
    }
}