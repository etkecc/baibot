pub trait ConfigTrait {
    fn validate(&self) -> Result<(), String>;
}
