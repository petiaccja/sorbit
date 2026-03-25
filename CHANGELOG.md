## 0.1.1 (2026-03-25)

### Features

- add derivation of PackInto and UnpackFrom for enumerations
- add feature to specify constant expression for the value of a struct field
- added `len` and `byte_count` attributes to store collection size explicitly
- added serialization of PhantomData + support for value=_ transforms
- added serialization of u128 and i128
- added support for fields in enum variants
- significant and breaking API changes to Serializer and Deserializer
