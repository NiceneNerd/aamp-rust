# Nintendo parameter archive (AAMP) library in Rust

A simple to use library for reading, writing, and converting Nintendo parameter archive (AAMP) files
in Rust. Supports only AAMP version 2, used in _The Legend of Zelda: Breath of the Wild_. Can
convert from AAMP to readable, editable YAML and back.

```rust
let mut file = File::open("Enemy_Lizalfos_Senior.baiprog").unwrap();
// Read an AAMP ParameterIO from any reader that implements Seek + Read
let pio = ParameterIO::from_binary(&mut file).unwrap();
for list in pio.lists() {
    // Do stuff with lists
}
for obj in pio.objects() {
    // Do stuff with objects
}
// Dumps YAML representation to a String
let yaml_dump: String = pio.to_text().unwrap();
```
