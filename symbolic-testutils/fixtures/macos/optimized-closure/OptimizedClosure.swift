public func makeClosure(_ offset: Int32) -> (Int32) -> Int32 {
    return { value in
        let scaled = value &* 3
        return scaled &+ offset
    }
}

@_cdecl("invoke_optimized_closure")
public func invokeOptimizedClosure(_ value: Int32, _ offset: Int32) -> Int32 {
    return makeClosure(offset)(value)
}
