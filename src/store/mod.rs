pub mod store {
    use std::{fs, io};

    use crate::store::hashlib::get_file_hash;

    pub struct Store {
        opts: StoreOpts,
    }

    pub struct StoreOpts {
        /** for configuring where the file is stored */
        root_dir: String,
        /** for handling how the file path should be transformed */
        filename_transform: PathTransformFn
    }

    impl Store {
        pub fn new(opts: StoreOpts) -> Store {
            Store {
                opts
            }
        }

        // pub fn read_stream(&self, key: String) 

        /** 
         * Write a stream to the store
         * @param key: the key to store the stream
         * @param r: the stream to store
         */
        pub fn write_stream(&self, key: String, r: &mut dyn io::Read) -> Result<(), io::Error> {
            // house keeping
            // create the directory if it doesn't exist
            fs::create_dir_all(&self.opts.root_dir).unwrap();
            let mut filename = (self.opts.filename_transform)(key);
            filename = format!("{}/{}", self.opts.root_dir, filename);
            
            let mut w = fs::File::create(&filename).unwrap();
            
            // write the stream to the file
            let bytes_written = io::copy(r, &mut w)?;
            println!("written {} bytes to {}", bytes_written, filename);

            Ok(())
        }
    }

    /** common interface for a path transform function */
    type PathTransformFn = fn(String) -> String;

    #[cfg(test)]
    mod tests {
        use crate::store::hashlib::filename_transform;

        use super::*;

        #[test]
        fn test_store_write_stream() {
            let store = Store { opts: StoreOpts { filename_transform: |s| s, root_dir: "test".to_string() } };
            let key = String::from  ("test");
            let mut r = io::Cursor::new(vec![1, 2, 3, 4]);
            let res = store.write_stream(key, &mut r);
            assert_eq!(res.is_ok(), true);
        }

        #[test]
        fn test_store_write_stream_with_path_transform() {
            let store = Store { opts: StoreOpts { filename_transform: filename_transform, root_dir: "test".to_string() } };
            let key = String::from("test");
            let mut r = io::Cursor::new(vec![1, 2, 3, 4]);
            let res = store.write_stream(key, &mut r);
            assert_eq!(res.is_ok(), true);
        }
    }
}

pub mod hashlib;