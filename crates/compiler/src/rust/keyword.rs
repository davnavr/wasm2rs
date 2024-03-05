macro_rules! keywords {
    ($($case:ident),*) => {
        /// List of Rust keywords.
        #[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
        #[allow(missing_docs)]
        #[non_exhaustive]
        pub enum Keyword {
            $($case),*
        }

        impl Keyword {
            #[allow(missing_docs)]
            pub const fn into_str(self) -> &'static str {
                match self {
                    $(Self::$case => {
                        const BYTES: &[u8] = stringify!($case).as_bytes();

                        const BYTES_LOWERCASE: [u8; BYTES.len()] = {
                            let mut bytes = [0u8; BYTES.len()];

                            let mut index = 0usize;
                            loop {
                                if index == BYTES.len() {
                                    break;
                                }

                                bytes[index] = BYTES[index].to_ascii_lowercase();
                                index += 1;
                            }

                            bytes
                        };

                        const NAME: &str = {
                            if let Ok(name) = std::str::from_utf8(&BYTES_LOWERCASE) {
                                name
                            } else {
                                panic!(concat!("invalid name for ", stringify!($case)))
                            }
                        };

                        NAME
                    },)*
                }
            }
        }
    }
}

keywords! {
    Mod
    // TODO: How will primtive types like i32 be modelled, Ident isn't appropriate
}

impl std::fmt::Display for Keyword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.into_str())
    }
}
