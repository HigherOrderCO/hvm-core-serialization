#![feature(slice_group_by)]
#![feature(is_sorted)]
#![doc = include_str!("../README.md")]
use bitbuffer::{BitRead, BitWrite, Endianness, LittleEndian};

pub mod net;
pub mod scalars;
pub mod tree;
pub mod wiring;

/// Encodes a tree/net/wiring into a byte vector using little endian
pub fn encode(value: &impl BitWrite<LittleEndian>) -> Vec<u8> {
  encode_endian(value, LittleEndian)
}

/// Decodes a tree/net/wiring from a byte slice using little endian
pub fn decode<'a, T: BitRead<'a, LittleEndian>>(bytes: &[u8]) -> T {
  decode_endian(bytes, LittleEndian)
}

pub fn encode_endian<E>(value: &impl BitWrite<E>, endianness: E) -> Vec<u8>
where
  E: Endianness,
{
  let mut write_bytes = vec![];
  let mut write_stream = bitbuffer::BitWriteStream::new(&mut write_bytes, endianness);
  write_stream.write(value).unwrap();
  write_bytes
}

pub fn decode_endian<'a, E, T>(bytes: &[u8], endianness: E) -> T
where
  E: Endianness,
  T: BitRead<'a, E>,
{
  let read_buffer = bitbuffer::BitReadBuffer::new_owned(bytes.to_vec(), endianness);
  let mut read_stream = bitbuffer::BitReadStream::new(read_buffer);
  read_stream.read::<T>().unwrap()
}

#[cfg(test)]
mod tests {
  use super::net::Net;
  use super::*;
  use bitbuffer::BigEndian;
  use hvmc::ast::do_parse_net;

  #[test]
  fn test_big_endian() {
    let net: Net = do_parse_net("a & (a *) ~ (b b)").into();

    let bytes = encode_endian(&net, BigEndian);
    let decoded_net: Net = decode_endian(&bytes, BigEndian);

    assert_eq!(net.normalize(), decoded_net.normalize());
  }
}
