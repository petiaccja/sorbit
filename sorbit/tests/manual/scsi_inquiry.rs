use sorbit::io::FixedMemoryStream;
use sorbit::ser_de::Serialize;
use sorbit::stream_ser_de::StreamSerializer;
use sorbit::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[sorbit(byte_order=big_endian)]
struct Inquiry {
    #[sorbit(bit_field=_byte0, repr=u8, bit_numbering=LSB0)]
    #[sorbit(bits=5..=7)]
    peripheral_qualifier: u8,
    #[sorbit(bit_field=_byte0, bits=0..=4)]
    peripheral_device_type: u8,

    #[sorbit(bit_field=_byte1, repr=u8, bit_numbering=LSB0, bits=7)]
    rmb: bool,

    version: u8,

    #[sorbit(bit_field=_byte3, repr=u8, bit_numbering=LSB0)]
    #[sorbit(bits = 5)]
    norm_aca: bool,
    #[sorbit(bit_field=_byte3, bits=4)]
    hi_sup: bool,
    #[sorbit(bit_field=_byte3, bits=0..=3)]
    response_data_format: u8,

    additional_length: u8,

    #[sorbit(bit_field=_byte5, repr=u8, bit_numbering=LSB0)]
    #[sorbit(bits = 7)]
    scss: bool,
    #[sorbit(bit_field=_byte5, bits=6)]
    acc: bool,
    #[sorbit(bit_field=_byte5, bits=4..=5)]
    tpgs: u8,
    #[sorbit(bit_field=_byte5, bits=3)]
    threepc: bool,
    #[sorbit(bit_field=_byte5, bits=0)]
    protect: bool,
    // This structure is incomplete
}

#[test]
fn serialize() {
    let input = Inquiry {
        peripheral_qualifier: 0,
        peripheral_device_type: 0,
        rmb: false,
        version: 0,
        norm_aca: false,
        hi_sup: false,
        response_data_format: 0,
        additional_length: 0,
        scss: false,
        acc: false,
        tpgs: 0,
        threepc: false,
        protect: false,
    };
    let mut buffer = [0u8; 36];
    let mut serializer = StreamSerializer::new(FixedMemoryStream::new(&mut buffer));
    input.serialize(&mut serializer).unwrap();
}
