use sorbit::{
    PackInto, UnpackFrom,
    bit::{PackInto, UnpackFrom},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PackInto, UnpackFrom)]
#[repr(u8)]
enum Strict {
    A = 0,
    B = 1,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PackInto, UnpackFrom)]
#[repr(u8)]
enum CatchAllEmpty {
    A = 0,
    #[sorbit(catch_all)]
    CatchAll,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PackInto, UnpackFrom)]
#[repr(u8)]
enum CatchAllTuple {
    A = 0,
    #[sorbit(catch_all)]
    CatchAll(u8),
}

#[cfg(test)]
mod tests {
    use super::*;

    use rstest::rstest;

    #[rstest]
    #[case(Strict::A, 0x00_u16)]
    #[case(Strict::B, 0x01_u16)]
    fn valid(#[case] value: Strict, #[case] packed: u16) {
        let forward: u16 = value.pack_into(2).unwrap();
        assert_eq!(forward, packed);
        let backward = Strict::unpack_from(packed, 2).unwrap();
        assert_eq!(value, backward);
    }

    #[test]
    fn invalid() {
        assert_eq!(Strict::unpack_from(3u16, 2), Err(3u16));
    }

    #[rstest]
    #[case(CatchAllEmpty::A, 0x00_u16)]
    #[case(CatchAllEmpty::CatchAll, 0x01_u16)]
    fn catch_all_empty(#[case] value: CatchAllEmpty, #[case] packed: u16) {
        let forward: u16 = value.pack_into(2).unwrap();
        assert_eq!(forward, packed);
        let backward = CatchAllEmpty::unpack_from(packed, 2).unwrap();
        assert_eq!(value, backward);
    }

    #[rstest]
    #[case(CatchAllTuple::A, 0x00_u16)]
    #[case(CatchAllTuple::CatchAll(1), 0x01_u16)]
    #[case(CatchAllTuple::CatchAll(3), 3_u16)]
    fn catch_all_tuple(#[case] value: CatchAllTuple, #[case] packed: u16) {
        let forward: u16 = value.pack_into(2).unwrap();
        assert_eq!(forward, packed);
        let backward = CatchAllTuple::unpack_from(packed, 2).unwrap();
        assert_eq!(value, backward);
    }
}
