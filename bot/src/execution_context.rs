use runtime::cpu::Cpu;

pub struct ExecutionContext {
  pub cpu: Option<Cpu>
}

impl ExecutionContext {
  pub fn new() -> Self {
    let mut this = ExecutionContext {
      cpu: None
    };
    this.init_ivt();
    this
  }

  pub fn init_ivt(&mut self) {

  }
}
