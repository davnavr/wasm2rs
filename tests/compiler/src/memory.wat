(module
  (memory (export "mem") 1)

  (data (memory 0) (offset i32.const 1234) "\01\02\03\04")

  (func (export "write_my_int")
    i32.const 500
    i32.const 65
    i32.store)

  (func (export "read_my_int") (result i32)
    i32.const 500
    i32.load)

  (func (export "out_of_bounds_read") (result i32)
    i32.const 65536
    i32.load)

  (func (export "grow_then_write") (result i32)
    (local $old_size i32)
    (local $address i32)
    (local.set $old_size (memory.size))
    (local.set $address (i32.add (i32.mul (local.get $old_size) (i32.const 65536)) (i32.const 16)))

    (drop (memory.grow (i32.add (i32.const 1) (local.get $old_size))))

    (i32.store (local.get $address) (i32.const 0xBABA))
    (i32.load (local.get $address)))
)
