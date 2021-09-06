# ☁ gm_async_write

Simple module that adds `file.AsyncWrite` and `file.AsyncAppend` to Garry's Mod.

These functions are mostly based off [`file.AsyncRead`](https://wiki.facepunch.com/gmod/file.AsyncRead) and have similar arguments and usage.

These will probably not be 1:1 with a native GLua implementation (if we ever get one...), so do be aware that migrating from this to a native implementation may be a breaking change.

# Installation

1. Download the module that corresponds to your Garry's Mod server's branch and operating system.
	* If your server is Linux and running the **main** (default) branch, you need `gmsv_async_write_linux.dll`
	* If your server is Windows and running the **main** (default) branch, you need `gmsv_async_write_win32.dll`
	* If your server is Linux and running the **x86-64** branch, you need `gmsv_async_write_linux64.dll`
	* If your server is Windows and running the **x86-64** branch, you need `gmsv_async_write_win64.dll`

2. Install it to `garrysmod/lua/bin/` (if the folder does not exist, create it)

You're done!

# Usage

See [`file.AsyncRead`](https://wiki.facepunch.com/gmod/file.AsyncRead) and [`FSASYNC`](https://wiki.facepunch.com/gmod/Enums/FSASYNC)

```lua
require("async_write")

local status = file.AsyncWrite("hello.txt", "Asynchronous write!", function(path, status)
	if status == FSASYNC_OK then
		print("OK!")
	elseif status == FSASYNC_ERR_FILEOPEN then
		print("Failed to open hello.txt")
	elseif status == FSASYNC_ERR_FAILURE then
		print("Failed to write to hello.txt")
	end
end)
print("Queued task, status: " .. status)

local status = file.AsyncAppend("hello.txt", "Asynchronous append! Who will win?", function(path, status)
	if status == FSASYNC_OK then
		print("OK!")
	elseif status == FSASYNC_ERR_FILEOPEN then
		print("Failed to open hello.txt")
	elseif status == FSASYNC_ERR_FAILURE then
		print("Failed to write to hello.txt")
	end
end)
print("Queued task, status: " .. status)
```