use kiwi_schema::Value;

pub trait FromKiwi {
    fn from_kiwi(value: &Value) -> Self;
}
