use bitbuffer::{BitRead, BitWrite, Endianness};

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

impl From<u32> for VarLenNumber {
  fn from(n: u32) -> Self {
    Self(n as u64)
  }
}

impl From<i64> for VarLenNumber {
  fn from(n: i64) -> Self {
    Self(n as u64)
  }
}

impl From<f32> for VarLenNumber {
  fn from(n: f32) -> Self {
    Self(n.to_bits() as u64)
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

impl From<VarLenNumber> for u32 {
  fn from(n: VarLenNumber) -> Self {
    n.0 as u32
  }
}

impl From<VarLenNumber> for i64 {
  fn from(n: VarLenNumber) -> Self {
    n.0 as i64
  }
}

impl From<VarLenNumber> for f32 {
  fn from(n: VarLenNumber) -> Self {
    f32::from_bits(n.0 as u32)
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
  // FIXME: use a table for storing the ref's strings
  REF(HVMRef),
  VAR,
  NUM(VarLenNumber),
  OPS,
  MAT,
  ERA,
  CON,
  DUP,
}

impl From<&hvm::ast::Tree> for Tag {
  fn from(value: &hvm::ast::Tree) -> Self {
    use hvm::ast::Tree::*;
    use Tag::*;
    match value {
      Era => ERA, //DynamicCtr((0u64.into(), 0u64.into())),
      Con { .. } => CON,
      Dup { .. } => DUP,
      Var { .. } => VAR, // incorrect, but we don't know the index yet
      Ref { nam } => REF(nam.clone().into()),
      Num { val } => NUM(val.0.into()),
      Opr { .. } => OPS,
      Swi { .. } => MAT,
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
