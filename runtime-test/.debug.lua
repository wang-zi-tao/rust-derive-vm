local dap = require("dap")
dap.run({
	name = "Launch",
	type = "lldb",
	request = "launch",
	program = "/home/wangzi/workspace/JVM/JVM/target/debug/deps/runtime_test-24abd696382f6a58",
	cwd = "${workspaceFolder}",
	-- stopOnEntry = true,
})
