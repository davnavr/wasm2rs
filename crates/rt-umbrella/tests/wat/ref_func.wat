(module
  (elem declare func $TheFunc)

  (func $TheFunc (param i32) (result i32 i32)
    local.get 0
    local.get 0
    i32.add
    local.get 0
    local.get 0
    i32.mul)

  (func (export "callTheFunc")
    ref.func $TheFunc
    ;; TODO: call_indirect
    unreachable))
