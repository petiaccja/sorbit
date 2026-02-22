# Sorbit


![Language](https://img.shields.io/badge/Language-Rust-blue)
![Language](https://img.shields.io/badge/License-MIT-blue)
[![build_and_test](https://github.com/petiaccja/sorbit/actions/workflows/build_and_test.yml/badge.svg)](https://github.com/petiaccja/sorbit/actions/workflows/build_and_test.yml)

Sorbit is a binary serialization framework that gives you complete control over the layout of the serialized data. Sorbit helps you define network packets, firmware messages, or other binary data structures that are governed by an external specification.

### How is Sorbit different from [serde](https://github.com/serde-rs/serde)?

Serde is a general purpose serialization framework that handles both text and binary formats, but the control over the exact layout of the binary formats is limited and more difficult to specify. Sorbit, on the other hand, is a specialized serialization framework with tightly controlled binary formats at its focus. While sorbit can be used as a general purpose framework, and can even serialize to text formats, you're better off using a framework like serde for that.

## Example

You can define the layout of data structure using attributes. This example shows the "inquiry" data format from the SCSI (hard drives, etc.) standard:

```rust
use sorbit::{Serialize, Deserialize};

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

    // The rest of the members hidden for brevity.
    // ...
}
```

You can then convert the structure into bytes that can be sent to the recipient (in this case a SCSI hard drive):

```rust
use sorbit::io::FixedMemoryStream;
use sorbit::serialize::{Serialize, StreamSerializer};

let inquiry = Inquiry::default();
let mut buffer = [0u8; 36];
let stream = FixedMemoryStream::new(&mut buffer); // no_std fixed size stream.
let mut serializer = StreamSerializer::new(stream);
inquiry.serialize(&mut serializer).unwrap();
```

## License

Sorbit is distributed under the MIT license, like most Rust libraries. Feel free to use sorbit both commercially and non-commercially.