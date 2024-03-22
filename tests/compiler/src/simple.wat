(module
  (func $add_five (export "add_five") (param i32) (result i32)
    local.get 0
    i32.const 5
    i32.add
    return)

  (func (export "add_fifteen") (param i32) (result i32)
    local.get 0
    call $add_five
    call $add_five
    call $add_five)

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

  (func (export "halt_on_even") (param i32)
    ;; Push a bunch of garbage on the stack
    i64.const 3252365
    i32.const 9932
    i64.const 3295

    block
      local.get 0
      loop (param i32)
        i32.const 2
        i32.rem_u
        i32.eqz
        if
          return
        end

        i32.const 1
        br 0
      end
    end
    unreachable)

  (global $kibibyte i32 (i32.const 1024))
  (func (export "size_of_kibibyte") (result i32)
    global.get $kibibyte)

  (global $counter (export "counter") (mut i32) (i32.const 0))
  (func (export "increment_counter")
    global.get $counter
    i32.const 1
    i32.add
    global.set $counter)
)
