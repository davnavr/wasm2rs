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

  (func (export "loop_factorial_unsigned") (param $n i32) (result i64)
    (local $acc i64)

    local.get $n
    i32.eqz
    if
      (return (i64.const 1))
    end

    (local.set $acc (i64.const 1))

    loop (result i64)
      (i64.mul (i64.extend_i32_u (local.get $n)) (local.get $acc))
      local.set $acc

      (i64.eq (i64.const 1) (i64.extend_i32_u (local.get $n)))
      if (result i64)
        local.get $acc
      else
        (i32.sub (local.get $n) (i32.const 1))
        local.set $n
        br 1
      end
    end)
)
