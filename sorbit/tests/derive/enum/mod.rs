use sorbit::ser_de::{Deserialize, Deserializer, Serialize, Serializer};

#[repr(u8)]
#[derive(Default)]
enum Test {
    #[default]
    A = 4,
    B(i32),
    C { c: i32 },
}

impl Serialize for Test {
    fn serialize<S: Serializer>(&self, serializer: &mut S) -> Result<S::Success, S::Error> {
        match self {
            Test::A => 4u8.serialize(serializer),
            Test::B(b) => 5u8.serialize(serializer).and_then(|_| b.serialize(serializer)),
            Test::C { c } => 6u8.serialize(serializer).and_then(|_| c.serialize(serializer)),
        }
    }
}

impl Deserialize for Test {
    fn deserialize<D: Deserializer>(deserializer: &mut D) -> Result<Self, D::Error> {
        let discriminant = u8::deserialize(deserializer)?;
        match discriminant {
            4 => Ok(Test::A),
            5 => {
                let b = i32::deserialize(deserializer)?;
                let value = Test::B { 0: b };
                Ok(value)
            }
            6 => {
                let c = i32::deserialize(deserializer)?;
                let value = Test::C { c };
                Ok(value)
            }
            _ => panic!(),
        }
    }
}
