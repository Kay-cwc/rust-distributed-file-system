use std::io;

/// given a stream, read all the bytes from the stream iteratively until EOF
/// and return the bytes read
pub fn read_all_from_stream(r: &mut dyn io::Read) -> io::Result<Vec<u8>> {
    let mut buf = vec![0; 1024];
    let mut result = Vec::new();

    loop {
        match r.read(&mut buf) {
            Ok(0) => break, // EOF
            Ok(n) => {
                result.extend_from_slice(&buf[..n]);
                buf.clear();
                continue
            }
            Err(e) => {
                println!("Error reading from stream: {}", e);
                return Err(e);
            },
        }
    };

    println!("Read {} bytes in total", result.len());

    Ok(result)
}