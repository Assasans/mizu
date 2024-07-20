<div align="center">

# Mizu

RISC-V sandbox for Discord bots.

</div>

## Reference

### Memory Map

| Start                | End                  | Size    | Description         | Type          |
|----------------------|----------------------|---------|---------------------|---------------|
| `0x0000000000010000` | `0x000000000002ffff` | 128 KiB | Hardware data area  |               |
| `0xffffffff80000000` | `0xffffffff87ffffff` | 128 MiB | Conventional memory | usable memory |

## License

Licensed under either of

* Apache License, Version 2.0
  ([LICENSE-APACHE](LICENSE-APACHE) or https://apache.org/licenses/LICENSE-2.0)
* MIT license
  ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you, as defined in the Apache-2.0 license, shall be
dual licensed as above, without any additional terms or conditions.
