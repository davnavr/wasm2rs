(module
  (func (export "add_five") (param i32) (result i32)
    local.get 0
    i32.const 5
    i32.add
    return)

  (func (export "block_me_up") (param i32) (result i32)
    block (result i32 i32)
      local.get 0
      local.get 0
    end
    i32.add)

  (func (export "unreachable_instruction") (result i32)
    unreachable)

  (func (export "life") (param i32) (result i32)
    local.get 0
    i32.const 42
    i32.eq
    if (result i32)
      i32.const 0x42424242
    else
      i32.const 0xDEAD
  	end)

  (func (export "do_nothing"))
)
