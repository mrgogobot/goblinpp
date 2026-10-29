//! Numeric/text NPY parsing without Python evaluation or pickle handling.
use super::{MAX_BYTES, MAX_ITEMS, fail, scalar};
use crate::error::Result;
use crate::evaluator::Value;

#[derive(Debug)]
pub struct Array<'a> {
    pub dtype: String,
    pub shape: Vec<usize>,
    pub fortran: bool,
    kind: u8,
    width: usize,
    little: bool,
    count: usize,
    data: &'a [u8],
}
struct Header<'a> {
    bytes: &'a [u8],
    at: usize,
}
impl Header<'_> {
    fn ws(&mut self) {
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_whitespace) {
            self.at += 1;
        }
    }
    fn eat(&mut self, byte: u8) -> bool {
        self.ws();
        if self.bytes.get(self.at) == Some(&byte) {
            self.at += 1;
            true
        } else {
            false
        }
    }
    fn need(&mut self, byte: u8) -> Result<()> {
        if self.eat(byte) {
            Ok(())
        } else {
            Err(fail(
                "Malformed NPY literal header; arbitrary expressions are refused.",
            ))
        }
    }
    fn text(&mut self) -> Result<String> {
        self.ws();
        let quote = *self
            .bytes
            .get(self.at)
            .ok_or_else(|| fail("Truncated NPY header."))?;
        if !matches!(quote, b'\'' | b'"') {
            return Err(fail(
                "NPY descr and keys must be literal strings; structured/object descriptors are unsupported.",
            ));
        }
        self.at += 1;
        let mut out = String::new();
        while let Some(&ch) = self.bytes.get(self.at) {
            self.at += 1;
            if ch == quote {
                return Ok(out);
            }
            if !(32..=126).contains(&ch) || ch == b'\\' {
                return Err(fail(
                    "Unsupported escaped/non-ASCII scalar NPY header string.",
                ));
            }
            out.push(ch as char);
        }
        Err(fail("Unclosed NPY header string."))
    }
    fn boolean(&mut self) -> Result<bool> {
        self.ws();
        for (s, v) in [(b"True".as_slice(), true), (b"False".as_slice(), false)] {
            if self.bytes[self.at..].starts_with(s) {
                self.at += s.len();
                return Ok(v);
            }
        }
        Err(fail("NPY fortran_order must be True or False."))
    }
    fn shape(&mut self) -> Result<Vec<usize>> {
        self.need(b'(')?;
        let mut out = Vec::new();
        if self.eat(b')') {
            return Ok(out);
        }
        loop {
            self.ws();
            let start = self.at;
            while self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) {
                self.at += 1;
            }
            if self.at == start {
                return Err(fail("NPY shape requires nonnegative literal integers."));
            }
            let n = std::str::from_utf8(&self.bytes[start..self.at])
                .unwrap()
                .parse::<usize>()
                .ok()
                .filter(|n| *n <= MAX_BYTES)
                .ok_or_else(|| fail("NPY axis exceeds the bounded reader's limit."))?;
            out.push(n);
            if out.len() > 8 {
                return Err(fail("NPY supports at most eight axes in this stage."));
            }
            if self.eat(b')') {
                if out.len() == 1 {
                    return Err(fail("One-axis NPY shape must be a tuple with a comma."));
                }
                break;
            }
            self.need(b',')?;
            if self.eat(b')') {
                break;
            }
        }
        Ok(out)
    }
}
impl<'a> Array<'a> {
    pub fn parse(bytes: &'a [u8]) -> Result<Self> {
        if bytes.len() > MAX_BYTES || bytes.len() < 10 || &bytes[..6] != b"\x93NUMPY" {
            return Err(fail("Invalid or oversized NPY magic/header."));
        }
        let (prefix, len) = match (bytes[6], bytes[7]) {
            (1, 0) => (
                10,
                usize::from(u16::from_le_bytes(bytes[8..10].try_into().unwrap())),
            ),
            (2 | 3, 0) if bytes.len() >= 12 => (
                12,
                u32::from_le_bytes(bytes[8..12].try_into().unwrap()) as usize,
            ),
            _ => return Err(fail("Supported NPY versions are 1.0, 2.0 and 3.0.")),
        };
        let end = prefix + len;
        if len == 0 || len > 65536 || end > bytes.len() || end % 16 != 0 || bytes[end - 1] != b'\n'
        {
            return Err(fail(
                "NPY header length, alignment or newline is invalid (64 KiB maximum).",
            ));
        }
        let mut p = Header {
            bytes: &bytes[prefix..end],
            at: 0,
        };
        p.need(b'{')?;
        let (mut dtype, mut shape, mut fortran) = (None, None, None);
        loop {
            if p.eat(b'}') {
                break;
            }
            let key = p.text()?;
            p.need(b':')?;
            match key.as_str() {
                "descr" if dtype.is_none() => dtype = Some(p.text()?),
                "shape" if shape.is_none() => shape = Some(p.shape()?),
                "fortran_order" if fortran.is_none() => fortran = Some(p.boolean()?),
                _ => return Err(fail("NPY header has duplicate or unsupported keys.")),
            }
            if p.eat(b'}') {
                break;
            }
            p.need(b',')?;
        }
        p.ws();
        if p.at != p.bytes.len() {
            return Err(fail("Unexpected expression after NPY header dictionary."));
        }
        let dtype = dtype.ok_or_else(|| fail("NPY descr missing."))?;
        let shape = shape.ok_or_else(|| fail("NPY shape missing."))?;
        let fortran = fortran.ok_or_else(|| fail("NPY fortran_order missing."))?;
        let d = dtype.as_bytes();
        if d.len() < 3 {
            return Err(fail(
                "Unsupported NPY dtype; object/pickle arrays are never read.",
            ));
        }
        let endian = d[0];
        let kind = d[1];
        if !matches!(endian, b'<' | b'>' | b'|') {
            return Err(fail(
                "NPY dtype needs explicit byte order (<, > or byte-order-independent |); native '=' is refused.",
            ));
        }
        if !d[2..].iter().all(u8::is_ascii_digit) {
            return Err(fail("Unsupported NPY scalar dtype descriptor."));
        }
        let mut width = dtype[2..]
            .parse::<usize>()
            .ok()
            .filter(|w| *w <= crate::text_runtime::MAX_TEXT_BYTES)
            .ok_or_else(|| fail("NPY item width exceeds limit."))?;
        let valid = match kind {
            b'b' => width == 1,
            b'i' | b'u' => matches!(width, 1 | 2 | 4 | 8),
            b'f' => matches!(width, 4 | 8),
            b'S' | b'U' => true,
            _ => false,
        };
        if !valid {
            return Err(fail(format!(
                "Unsupported NPY dtype {dtype:?}; pickle/object, structured, complex and extended types are refused."
            )));
        }
        if endian == b'|'
            && !(matches!(kind, b'S' | b'b') || width == 1 && matches!(kind, b'i' | b'u'))
        {
            return Err(fail(
                "Byte-order-independent NPY marker is invalid for this multibyte dtype.",
            ));
        }
        if kind == b'U' {
            width = width
                .checked_mul(4)
                .ok_or_else(|| fail("NPY Unicode width overflow."))?;
        }
        let count = shape
            .iter()
            .try_fold(1usize, |a, b| a.checked_mul(*b))
            .ok_or_else(|| fail("NPY shape product overflows."))?;
        let length = count
            .checked_mul(width)
            .ok_or_else(|| fail("NPY payload size overflows."))?;
        if end.checked_add(length) != Some(bytes.len()) {
            return Err(fail(
                "NPY data size does not match shape/dtype; truncated and trailing payloads are refused.",
            ));
        }
        Ok(Self {
            dtype,
            shape,
            fortran,
            kind,
            width,
            little: endian != b'>',
            count,
            data: &bytes[end..],
        })
    }
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }
    pub fn len(&self) -> usize {
        self.count
    }
    fn storage_index(&self, mut logical: usize) -> usize {
        if !self.fortran || self.shape.len() < 2 {
            return logical;
        }
        let mut coordinates = [0usize; 8];
        for axis in (0..self.shape.len()).rev() {
            coordinates[axis] = logical % self.shape[axis];
            logical /= self.shape[axis];
        }
        let (mut offset, mut stride) = (0, 1);
        for (axis, n) in self.shape.iter().enumerate() {
            offset += coordinates[axis] * stride;
            stride *= n;
        }
        offset
    }
    fn item(&self, index: usize) -> &[u8] {
        let start = self.storage_index(index) * self.width;
        &self.data[start..start + self.width]
    }
    fn unsigned(&self, index: usize) -> u64 {
        let b = self.item(index);
        let mut out = [0u8; 8];
        if self.little {
            out[..b.len()].copy_from_slice(b);
            u64::from_le_bytes(out)
        } else {
            out[8 - b.len()..].copy_from_slice(b);
            u64::from_be_bytes(out)
        }
    }
    pub fn integer(&self, index: usize) -> Result<i128> {
        if index >= self.count || !matches!(self.kind, b'i' | b'u') {
            return Err(fail("An integer NPY array is required."));
        }
        let v = self.unsigned(index);
        Ok(if self.kind == b'i' {
            let shift = 64 - self.width * 8;
            (((v << shift) as i64) >> shift) as i128
        } else {
            i128::from(v)
        })
    }
    pub fn indices(&self, limit: usize) -> Result<Vec<usize>> {
        if self.shape.len() != 1 || self.count > limit {
            return Err(fail(
                "Sparse index arrays must be bounded one-dimensional integer arrays.",
            ));
        }
        (0..self.count)
            .map(|i| {
                usize::try_from(self.integer(i)?)
                    .map_err(|_| fail("Sparse index is negative or outside usize range."))
            })
            .collect()
    }
    fn string(&self, index: usize) -> Result<String> {
        let bytes = self.item(index);
        let out = if self.kind == b'S' {
            let length = bytes.iter().rposition(|b| *b != 0).map_or(0, |i| i + 1);
            std::str::from_utf8(&bytes[..length])
                .map_err(|_| fail("NPY byte string is not valid UTF-8; no lossy conversion."))?
                .to_owned()
        } else {
            let mut codes = Vec::new();
            for ch in bytes.chunks_exact(4) {
                codes.push(if self.little {
                    u32::from_le_bytes(ch.try_into().unwrap())
                } else {
                    u32::from_be_bytes(ch.try_into().unwrap())
                });
            }
            while codes.last() == Some(&0) {
                codes.pop();
            }
            codes
                .into_iter()
                .map(|c| {
                    char::from_u32(c).ok_or_else(|| fail("Invalid Unicode scalar in NPY string."))
                })
                .collect::<Result<String>>()?
        };
        crate::text_runtime::bounded(out).map_err(fail)
    }
    pub fn values(&self, text: bool) -> Result<Vec<Value>> {
        if self.count > MAX_ITEMS {
            return Err(fail(format!(
                "NPY materialization exceeds {MAX_ITEMS} items; no flattening/truncation is implicit."
            )));
        }
        (0..self.count).map(|index|{
            if matches!(self.kind,b'S'|b'U'){return self.string(index).map(Value::Text);}
            if text {
                if matches!(self.kind,b'i'|b'u'){return Ok(Value::Text(self.integer(index)?.to_string()));}
                return Err(fail("npy_text/npz_text accepts only integer or UTF-8/Unicode string dtypes."));
            }
            let value=match self.kind {
                b'b'=>return match self.unsigned(index){0=>Ok(Value::Bool(false)),1=>Ok(Value::Bool(true)),_=>Err(fail(format!("Invalid boolean at logical element {index}.")))},
                b'i'|b'u'=>{let v=self.integer(index)?;if !(-9_007_199_254_740_991..=9_007_199_254_740_991).contains(&v){return Err(fail(format!("Integer at logical element {index} exceeds exact numeric range; use npy_text/npz_text for catalogue IDs.")));}v as f64},
                b'f' if self.width==4=>f64::from(f32::from_bits(self.unsigned(index) as u32)),
                b'f'=>f64::from_bits(self.unsigned(index)),
                _=>unreachable!(),
            };
            if !value.is_finite(){return Err(fail(format!("Nonfinite NPY number at logical element {index}; no rows skipped or invented zeros.")));}
            scalar(value)
        }).collect()
    }
}
