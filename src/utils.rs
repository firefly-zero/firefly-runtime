use alloc::vec::Vec;

pub(crate) fn read_all<R, E>(mut stream: R) -> Result<Vec<u8>, E>
where
    R: embedded_io::Read<Error = E>,
    E: embedded_io::Error,
{
    const CHUNK_SIZE: usize = 64;
    let mut result = Vec::new();
    let mut filled_size = 0;
    loop {
        result.resize(filled_size + CHUNK_SIZE, 0);
        let gained_size = stream.read(&mut result[filled_size..])?;
        if gained_size == 0 {
            break;
        }
        filled_size += gained_size;
    }
    result.truncate(filled_size);
    result.shrink_to_fit();
    Ok(result)
}

/// Read stream into the buffer.
///
/// Returns the number of bytes read.
pub(crate) fn read_into<R, E>(mut stream: R, buf: &mut [u8]) -> Result<usize, E>
where
    R: embedded_io::Read<Error = E>,
    E: embedded_io::Error,
{
    let mut buf = buf;
    let mut filled = 0;
    while !buf.is_empty() {
        let n = stream.read(buf)?;
        if n == 0 {
            break;
        }
        filled += n;
        buf = &mut buf[n..];
    }
    Ok(filled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_all() {
        let given = alloc::vec![1, 2, 3, 4];
        let res = read_all(&given[..]).unwrap();
        assert_eq!(res, given);
    }
}
