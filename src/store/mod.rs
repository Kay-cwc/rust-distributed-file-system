pub mod store {
    use std::{fs, io::{self, BufReader, ErrorKind}};

    pub struct Store {
        opts: StoreOpts,
    }

    pub struct StoreOpts {
        /// for configuring where the file is stored
        pub root_dir: String,
        /// for handling how the filename should be transformed. 
        /// @see hashlib::filename_transform for an example of transforming the filename from a key to a sha1 hash
        pub filename_transform: PathTransformFn
    }

    impl Store {
        pub fn new(opts: StoreOpts) -> Store {
            Store {
                opts
            }
        }

        /// given a key, return the file buffer
        pub fn read(&self, key: String) -> Result<Vec<u8>, ErrorKind> {
            let mut reader = self.read_stream(key)?;
            let mut buf = Vec::new();
            reader.read_to_end(&mut buf).unwrap();

            Ok(buf)
        }

        /// write the stream to the store
        pub fn write(&self, key: String, r: &mut dyn io::Read) -> Result<(), io::Error> {
            self.write_stream(key, r)
        }

        /// delete the file with the given key
        pub fn delete(&self, key: String) -> Result<(), ErrorKind> {
            let filename = self.fullpath(key);
            match fs::metadata(&filename) {
                Ok(_) => (),
                Err(_) => return Err(ErrorKind::NotFound)
            };
            match fs::remove_file(&filename) {
                Ok(_) => Ok(()),
                Err(e) => return Err(e.kind())
            }
        }

        /// clear the store directory
        pub fn clear(&self) -> Result<(), ErrorKind> {
            match fs::remove_dir_all(&self.opts.root_dir) {
                Ok(_) => Ok(()),
                Err(e) => return Err(e.kind())
            }
        }

        /// return a stream to the file
        fn read_stream(&self, key: String) -> Result<Box<dyn io::Read>, ErrorKind> {
            let filename = self.fullpath(key);
            let file = match fs::File::open(&filename) {
                Ok(f) => f,
                Err(_) => return Err(ErrorKind::NotFound)
            };
            let buf_reader = BufReader::new(file);
            
            Ok(Box::new(buf_reader))
        }

        /// Write a stream to the store  
        /// param key: the key to store the stream  
        /// param r: the stream to store
        fn write_stream(&self, key: String, r: &mut dyn io::Read) -> Result<(), io::Error> {
            // house keeping
            // create the directory if it doesn't exist
            fs::create_dir_all(&self.opts.root_dir).unwrap();
            let filename = self.fullpath(key);
            
            let mut w = fs::File::create(&filename).unwrap();
            
            // write the stream to the file
            let bytes_written = io::copy(r, &mut w)?;
            println!("written {} bytes to {}", bytes_written, filename);

            Ok(())
        }

        fn fullpath(&self, key: String) -> String {
            let mut filename = (self.opts.filename_transform)(key);
            filename = format!("{}/{}", self.opts.root_dir, filename);

            filename
        }
    }

    /** common interface for a path transform function */
    type PathTransformFn = fn(String) -> String;

    #[cfg(test)]
    mod tests {
        use crate::store::hashlib::filename_transform;

        use super::*;

        // the root directory for testing. avoid using the same root directory for other tests
        const TEST_ROOT_DIR: &str = "test_store";

        #[test]
        fn test_store_write_stream() {
            let store = Store { opts: StoreOpts { filename_transform: |s| s, root_dir: TEST_ROOT_DIR.to_string() } };
            let key = String::from  ("test");
            let mut r = io::Cursor::new(vec![1, 2, 3, 4]);
            let res = store.write_stream(key, &mut r);
            assert_eq!(res.is_ok(), true);
        }

        #[test]
        fn test_store_write_stream_with_path_transform() {
            let store = Store { opts: StoreOpts { filename_transform: filename_transform, root_dir: TEST_ROOT_DIR.to_string() } };
            let key = String::from("test");
            let mut r = io::Cursor::new(vec![1, 2, 3, 4]);
            let res = store.write_stream(key, &mut r);
            assert_eq!(res.is_ok(), true);
        }

        #[test]
        fn test_store_read_stream() {
            let store = Store { opts: StoreOpts { filename_transform: |s| s, root_dir: TEST_ROOT_DIR.to_string() } };
            let key = String::from("test");
            let mut r = io::Cursor::new(vec![1, 2, 3, 4]);
            store.write_stream(key.clone(), &mut r).unwrap();
            let res = store.read(key).unwrap();
            let expected_res = vec![1, 2, 3, 4];

            assert_eq!(res, expected_res);
        }

        #[test]
        fn test_store_read_unmatched_content() {
            let store = Store { opts: StoreOpts { filename_transform: |s| s, root_dir: TEST_ROOT_DIR.to_string() } };
            let key = String::from("test");
            let mut r = io::Cursor::new(vec![]);
            store.write_stream(key.clone(), &mut r).unwrap();
            let res = store.read(key).unwrap();

            assert_ne!(res, vec![1, 2, 3, 4]);
        }

        #[test]
        fn test_store_file_not_found() {
            let store = Store { opts: StoreOpts { filename_transform: |s| s, root_dir: TEST_ROOT_DIR.to_string() } };
            let key = String::from("some_non_existent_file_key");
            let res = store.read(key);

            assert_eq!(res.is_err(), true);
            assert!(res.unwrap_err() == ErrorKind::NotFound);
        }

        #[test]
        fn test_delete_file() {
            let store = Store { opts: StoreOpts { filename_transform: |s| s, root_dir: TEST_ROOT_DIR.to_string() } };
            let key = String::from("file_to_be_deleted");
            let mut r = io::Cursor::new(vec![1, 2, 3, 4]);
            store.write_stream(key.clone(), &mut r).unwrap();
            let res = store.delete(key).unwrap();

            assert_eq!(res, ());
        }

        #[test]
        fn test_delete_non_existent_file() {
            let store = Store { opts: StoreOpts { filename_transform: |s| s, root_dir: TEST_ROOT_DIR.to_string() } };
            let key = String::from("non_existent_file");
            let res = store.delete(key);

            assert_eq!(res.is_err(), true);
            assert!(res.unwrap_err() == ErrorKind::NotFound);
        }

        #[test]
        fn test_clear_store() {
            let store = Store { opts: StoreOpts { filename_transform: |s| s, root_dir: TEST_ROOT_DIR.to_string() } };
            let key = String::from("file_to_be_deleted");
            let mut r = io::Cursor::new(vec![1, 2, 3, 4]);
            store.write_stream(key.clone(), &mut r).unwrap();
            let res = store.clear().unwrap();

            assert_eq!(res, ());
        }
    }
}

pub mod hashlib;