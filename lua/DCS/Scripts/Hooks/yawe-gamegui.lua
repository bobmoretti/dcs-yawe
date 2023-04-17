local function init()
    log.write("[yawe-hook]", log.INFO, "Initializing ...")
    -- load settings from `Saved Games/DCS/Config/dcs-grpc.lua`

    if not yawe_config then
        _G.yawe_config = {
            dll_path = [[F:\projects\dcs\yawe\target\release\]],
            lua_path = [[F:\projects\dcs\yawe\lua\]],
            debug = true
        }
    end

    do
        log.write("[yawe-hook]", log.INFO, "Checking optional config at `Config/yawe-config.lua` ...")
        local file, err = io.open(lfs.writedir() .. [[Config\yawe-config.lua]], "r")
        if file then
            local f = assert(loadstring(file:read("*all")))
            setfenv(f, yawe_config)
            f()
            log.write("[yawe-hook]", log.INFO, "`Config/yawe-config.lua` successfully read")
        else
            log.write("[yawe-hook]", log.INFO, "`Config/yawe-config.lua` not found (" .. tostring(err) .. ")")
        end
    end

    dofile(yawe_config.lua_path .. [[hook.lua]])
    log.write("[yawe-hook]", log.INFO, "Initialized...")
end

local ok, err = pcall(init)
if not ok then
    log.write("[yawe-hook]", log.ERROR, "Failed to Initialize: " .. tostring(err))
end
