//! thin wapper of failsafe-rs to provide circuit breaker captilites to bb8.
//!
//! # Example
//!
//! Using an imaginary "foodb" database.
//!
//! ```ignore
//! #[tokio::main]
//! async fn main() {
//!     let manager = bb8_foodb::FooConnectionManager::new("localhost:1234");
//!     let circuitbreaker = bb8_failsafe::failsafe::Config::new().build();
//!     let safemanager = bb8_failsafe::FailsafeConnectionManager::new(manager, circuitbreaker);
//!     let pool = bb8::Pool::builder().build(safemanager).await.unwrap();
//!
//!     for _ in 0..20 {
//!         let pool = pool.clone();
//!         tokio::spawn(async move {
//!             let conn = pool.get().await.unwrap();
//!             // use the connection
//!             // it will be returned to the pool when it falls out of scope.
//!         });
//!     }
//! }
//! ```
use async_trait::async_trait;
pub use failsafe;
use failsafe::futures::CircuitBreaker;

/// A genric bb8::ConnectionManager wrapped in failsafe-rs
#[derive(Clone)]
pub struct FailsafeConnectionManager<T, U>
where
    T: bb8::ManageConnection,
    U: CircuitBreaker + std::marker::Send + std::marker::Sync + 'static,
{
    connection_manager: T,
    circuit_breaker: U,
}

impl<T, U> FailsafeConnectionManager<T, U>
where
    T: bb8::ManageConnection,
    U: CircuitBreaker + std::marker::Send + std::marker::Sync + 'static,
{
    /// Create a new FailsafeConnectionManager consuming a ConnectionManager and CircuitBreaker
    pub fn new(connection_manager: T, circuit_breaker: U) -> FailsafeConnectionManager<T, U> {
        FailsafeConnectionManager {
            connection_manager,
            circuit_breaker,
        }
    }
}

#[async_trait]
impl<T, U> bb8::ManageConnection for FailsafeConnectionManager<T, U>
where
    T: bb8::ManageConnection,
    U: CircuitBreaker + std::marker::Send + std::marker::Sync + 'static,
{
    type Connection = T::Connection;
    type Error = failsafe::Error<T::Error>;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        self.circuit_breaker
            .call(self.connection_manager.connect())
            .await
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        self.circuit_breaker
            .call(self.connection_manager.is_valid(conn))
            .await
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        self.connection_manager.has_broken(conn)
    }
}

#[cfg(test)]
mod tests {
    use crate::FailsafeConnectionManager;
    use async_trait::async_trait;
    use bb8::ManageConnection;
    use std::sync::{Arc, Mutex};
    use tokio::runtime::Runtime;

    #[derive(Clone)]
    struct FoobarConnectionManager {
        counter: Arc<Mutex<u32>>,
    }

    impl FoobarConnectionManager {
        fn new() -> FoobarConnectionManager {
            FoobarConnectionManager {
                counter: Arc::new(Mutex::new(0)),
            }
        }
    }

    #[async_trait]
    impl bb8::ManageConnection for FoobarConnectionManager {
        type Connection = ();
        type Error = ();

        async fn connect(&self) -> Result<Self::Connection, Self::Error> {
            let mut guard = self.counter.lock().unwrap();
            *guard = *guard + 1;
            if *guard > 3 {
                return Err(());
            }
            return Ok(());
        }

        async fn is_valid(&self, _conn: &mut Self::Connection) -> Result<(), Self::Error> {
            Ok(())
        }

        fn has_broken(&self, _conn: &mut Self::Connection) -> bool {
            false
        }
    }

    #[test]
    fn simple() {
        let circuit_breaker = failsafe::Config::new().build();
        let foomanager = FoobarConnectionManager::new();

        let rt = Runtime::new().unwrap();
        let failsafemanager = FailsafeConnectionManager::new(foomanager, circuit_breaker);

        rt.block_on(async {
            for _ in 0..3 {
                assert!(failsafemanager.connect().await.is_ok());
            }

            for _ in 4..5 {
                match failsafemanager.connect().await {
                    Ok(_) => panic!(),
                    Err(e) => match e {
                        failsafe::Error::Rejected => panic!(),
                        failsafe::Error::Inner(_) => {}
                    },
                }
            }

            for _ in 5..10 {
                match failsafemanager.connect().await {
                    Ok(_) => panic!(),
                    Err(e) => match e {
                        failsafe::Error::Rejected => {}
                        failsafe::Error::Inner(_) => (),
                    },
                }
            }
        });
    }
}
