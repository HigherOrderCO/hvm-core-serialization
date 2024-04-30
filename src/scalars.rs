use bitbuffer::{BitRead, BitWrite, Endianness};
use serde::de::IntoDeserializer;

#[derive(Debug, BitRead, BitWrite, PartialEq, Eq, Clone)]
pub struct HVMRef(pub String);

impl From<String> for HVMRef {
  fn from(n: String) -> Self {
    Self(n)
  }
}

impl From<HVMRef> for String {
  fn from(n: HVMRef) -> Self {
    n.0
  }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
/// Uses Elias gamma encoding
pub struct VarLenNumber(u64);

impl From<u64> for VarLenNumber {
  fn from(n: u64) -> Self {
    Self(n)
  }
}

impl From<i64> for VarLenNumber {
  fn from(n: i64) -> Self {
    Self(n as u64)
  }
}

impl From<f32> for VarLenNumber {
  fn from(n: f32) -> Self {
    // TODO: completely wrong fix me
    Self(n as u64)
  }
}

impl From<u16> for VarLenNumber {
  fn from(n: u16) -> Self {
    Self(n as u64)
  }
}

impl From<VarLenNumber> for u64 {
  fn from(n: VarLenNumber) -> Self {
    n.0
  }
}

impl From<VarLenNumber> for i64 {
  fn from(n: VarLenNumber) -> Self {
    n.0 as i64
  }
}

impl From<VarLenNumber> for f32 {
  fn from(n: VarLenNumber) -> Self {
    // TODO: completely wrong fix me
    n.0 as f32
  }
}

impl From<VarLenNumber> for u16 {
  fn from(n: VarLenNumber) -> Self {
    n.0 as u16
  }
}

impl From<VarLenNumber> for u8 {
  fn from(n: VarLenNumber) -> Self {
    n.0 as u8
  }
}

impl<E: Endianness> BitWrite<E> for VarLenNumber {
  fn write(&self, stream: &mut bitbuffer::BitWriteStream<E>) -> bitbuffer::Result<()> {
    // Add 1 to the number so that 0 is not a special case
    let n = self.0 + 1;
    // Write in unary, the number of bits(-1) needed to represent the number
    let bits = n.ilog2();
    for _ in 0..bits {
      stream.write_bool(false)?;
    }
    // Write the number in binary
    for i in (0..bits + 1).rev() {
      let bit = n & (1 << i) != 0;
      stream.write_bool(bit)?;
    }
    Ok(())
  }
}

impl<E: Endianness> BitRead<'_, E> for VarLenNumber {
  fn read(stream: &mut bitbuffer::BitReadStream<'_, E>) -> bitbuffer::Result<Self> {
    let mut bits = 0;
    // Read back in unary, the number of bits(-1) needed to represent the number
    while !stream.read_bool()? {
      bits += 1;
    }

    // Because we read one too many(one true bit)
    let mut n = 1;

    // Reconstruct the number from the binary representation
    for _ in 0..bits {
      n *= 2;
      if stream.read_bool()? {
        n += 1;
      }
    }

    // Subtract 1 to undo the +1 in the write function
    Ok(Self(n - 1))
  }
}

#[derive(Clone, PartialEq, Eq, Debug, BitWrite, BitRead)]
#[discriminant_bits = 3]
#[allow(clippy::upper_case_acronyms)]
pub enum Tag {
  ERA,
  // FIXME: use a table for storing the ref's strings
  REF(HVMRef),
  VAR,
  NUM((bool, VarLenNumber)),
  #[size = 5] // 1 extra bit for signaling if it's OP1 or OP2
  OPS(u32),
  MAT,
  CTR((VarLenNumber, VarLenNumber)),
}

impl From<&hvmc::ast::Tree> for Tag {
  fn from(value: &hvmc::ast::Tree) -> Self {
    use hvmc::ast::Tree::*;
    use Tag::*;
    match value {
      Era => ERA,
      &Ctr { lab, ref ports } => CTR(((ports.len() as u64).into(), lab.into())),
      Var { .. } => VAR, // incorrect, but we don't know the index yet
      Ref { nam } => REF(nam.clone().into()),
      // &Num { val } => NUM(val.into()),
      // &Op1 { opr, .. } => OPS(u16::from(opr) as u32 | 0b10000), // set 5th bit to 1
      // &Op2 { opr, .. } => OPS(u16::from(opr) as u32),
      Mat { .. } => MAT,
      &Int { val } => NUM((false, val.into())),
      F32 { val } => NUM((true, todo!())),
      Op { op, rhs, out } => todo!(),
      Adt {
        lab,
        variant_index,
        variant_count,
        fields,
      } => todo!(),
    }
  }
}

#[cfg(test)]
mod tests {
  use super::super::{decode, encode};
  use super::*;

  #[test]
  fn test_varlen_number() {
    for n in 0..35 {
      let n = VarLenNumber(n);
      let bytes = encode(&n);
      let decoded_n: VarLenNumber = decode(&bytes);
      assert_eq!(n, decoded_n);
    }
  }
}
