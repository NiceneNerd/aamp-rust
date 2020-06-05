use binread::BinRead;
use binwrite::BinWrite;

#[derive(BinRead, Debug, BinWrite, PartialEq, Clone, Copy)]
#[binwrite(little)]
pub struct Vec2(pub [f32; 2]);
#[derive(BinRead, Debug, BinWrite, PartialEq, Clone, Copy)]
#[binwrite(little)]
pub struct Vec3(pub [f32; 3]);
#[derive(BinRead, Debug, BinWrite, PartialEq, Clone, Copy)]
#[binwrite(little)]
pub struct Vec4(pub [f32; 4]);
#[derive(BinRead, Debug, BinWrite, PartialEq, Clone, Copy)]
#[binwrite(little)]
pub struct Color(pub [f32; 4]);
#[derive(BinRead, Debug, BinWrite, PartialEq, Clone, Copy)]
#[binwrite(little)]
pub struct Quat(pub [f32; 4]);

#[derive(BinRead, Debug, Default, PartialEq, Clone, BinWrite)]
pub struct Curve {
    pub a: u32,
    pub b: u32,
    #[binread(count = 30)]
    pub floats: Vec<f32>,
}

#[derive(BinRead, Debug, PartialEq, Clone, BinWrite)]
#[binwrite(little)]
pub struct Curve1 {
    pub curve: Curve,
}
#[derive(BinRead, Debug, PartialEq, Clone, BinWrite)]
#[binwrite(little)]
pub struct Curve2 {
    pub curve1: Curve,
    pub curve2: Curve,
}
#[derive(BinRead, Debug, PartialEq, Clone, BinWrite)]
#[binwrite(little)]
pub struct Curve3 {
    pub curve1: Curve,
    pub curve2: Curve,
    pub curve3: Curve,
}
#[derive(BinRead, Debug, PartialEq, Clone, BinWrite)]
#[binwrite(little)]
pub struct Curve4 {
    pub curve1: Curve,
    pub curve2: Curve,
    pub curve3: Curve,
    pub curve4: Curve,
}

#[derive(BinRead, Debug, BinWrite, PartialEq, Clone)]
#[binwrite(little)]
pub struct BufferInt {
    pub buffer: Vec<i32>,
}
#[derive(BinRead, Debug, BinWrite, PartialEq, Clone)]
#[binwrite(little)]
pub struct BufferF32 {
    pub buffer: Vec<f32>,
}
#[derive(BinRead, Debug, BinWrite, PartialEq, Clone)]
#[binwrite(little)]
pub struct BufferU32 {
    pub buffer: Vec<u32>,
}
#[derive(BinRead, Debug, BinWrite, PartialEq, Clone)]
#[binwrite(little)]
pub struct BufferBinary {
    pub buffer: Vec<u8>,
}
