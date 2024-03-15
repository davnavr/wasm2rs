(module
  (func (export "add_five") (param i32) (result i32)
    local.get 0
    i32.const 5
    i32.add
    return)
)
