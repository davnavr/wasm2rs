(module
  (memory 1)
  (func (export "write_my_int")
    i32.const 500
    i32.const 65
    i32.store)

  (func (export "read_my_int") (result i32)
    i32.const 500
    i32.load))
