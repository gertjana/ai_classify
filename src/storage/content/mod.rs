pub mod filesystem;
pub mod redis;
pub mod s3;

// #[cfg(test)]
// mod filesystem_test;

#[cfg(test)]
mod s3_test;

#[cfg(test)]
mod redis_test;

// Other content storage implementations can be added here
