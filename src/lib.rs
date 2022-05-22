pub use failsafe::futures::CircuitBreaker;
pub use failsafe::Error;
use async_trait::async_trait;

#[derive(Clone)]
pub struct FailsafeConnectionManager<T, U>
where
    T: bb8::ManageConnection,
    U: CircuitBreaker + std::marker::Send + std::marker::Sync + 'static,
{
    connection_manager: T,
    circuit_breaker: U,
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
        self.circuit_breaker.call(self.connection_manager.connect()).await
    }

    async fn is_valid(&self, conn: &mut Self::Connection) -> Result<(), Self::Error> {
        self.circuit_breaker.call(self.connection_manager.is_valid(conn)).await
    }

    fn has_broken(&self, conn: &mut Self::Connection) -> bool {
        self.connection_manager.has_broken(conn)
    }
}