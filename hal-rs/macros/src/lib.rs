use proc_macro::TokenStream;
use darling::ast::NestedMeta;
use darling::{Error, FromMeta};
use quote::{format_ident, quote};
use syn::{ItemFn, parse_macro_input};

#[derive(Debug, FromMeta)]
struct IsrArgs {
  code: u64,
}

#[proc_macro_attribute]
pub fn isr(args: TokenStream, item: TokenStream) -> TokenStream {
  let item = parse_macro_input!(item as ItemFn);

  let attr_args = match NestedMeta::parse_meta_list(args.into()) {
    Ok(args) => args,
    Err(error) => return TokenStream::from(Error::from(error).write_errors()),
  };

  let args = match IsrArgs::from_list(&attr_args) {
    Ok(args) => args,
    Err(error) => return TokenStream::from(error.write_errors())
  };

  let code = args.code;

  let isr_handler_ident = &item.sig.ident;
  let isr_wrapper_ident = format_ident!("_isr_{}", isr_handler_ident);

  (quote! {
    #[inline(never)]
    #item

    unsafe fn #isr_wrapper_ident() {
      let registers = ::mizu_hal::ivt::__save_registers();
      #isr_handler_ident(registers);
      ::core::arch::asm!("mret");
    }

    ::core::arch::global_asm!(
      ".section .text.ivt",
      concat!(".org .text.ivt + ", #code, " * 4"),
      "jal {}",
      sym #isr_wrapper_ident
    );
  }).into()
}
