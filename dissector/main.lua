local proto_rustudps = Proto("rustudps", "Rust UDP Sender")

local field_version = ProtoField.uint16("rustudps.version", "Version", base.DEC)
local field_name = ProtoField.string("rustudps.name", "Name")
local field_length = ProtoField.uint32("rustudps.length", "Data length", base.DEC)
local field_hash = ProtoField.bytes("rustudps.hash", "Hash")
local field_data = ProtoField.bytes("rustudps.data", "Data (messagepack)")

proto_rustudps.fields = { field_version, field_name, field_data, field_length, field_hash }

function proto_rustudps.dissector(buffer, pinfo, tree)
    -- check that the first 8 bytes are "RustUDPs"
    if buffer(0, 8):string() ~= "RustUDPs" then
        return false
    end

    pinfo.cols.protocol = "RustUDPs"
    local subtree = tree:add(proto_rustudps, buffer(), "Rust UDP Sender")
    local C = 8;

    -- The name is a null-terminated string
    local name = buffer(8):stringz()
    subtree:add(field_name, buffer(C, #name + 1), name)
    C = C + #name + 1

    -- The version is a 16-bit unsigned integer
    subtree:add(field_version, buffer(C, 2))
    C = C + 2

    -- The length is a 16-bit unsigned integer
    local length = buffer(C, 2):uint()
    subtree:add(field_length, buffer(C, 2), length)
    C = C + 2

    -- The hash is a 32-byte array
    subtree:add(field_hash, buffer(C, 32))
    C = C + 32

    -- The data is a messagepack array
    subtree:add(field_data, buffer(C, length))
    C = C + length

    --[[
    -- Parse the messagepack data
    local mp_tree = subtree:add(buffer(C-length, length), "MessagePack data")
    -- get data as a string
    local mp_data = buffer(C-length, length):string()
    local mp_module = require("MessagePack")
    local mp_parsed = mp_module.unpack(mp_data)
    mp_tree:add(mp_data, mp_parsed)
    --]]

    return true

end

udp_table = DissectorTable.get("udp.port"):add(1337, proto_rustudps)