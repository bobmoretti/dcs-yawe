local function writeLog(level, message)
    log.write("[yawe-hook]", level, message)
end

-- Register Callbacks in DCS World GUI environment

local yaweCallbacks = {}
YAWE = {}
local function onMissionLoadEnd()
    writeLog(log.INFO, "On Mission load end!")
    -- Let DCS know where to find the DLLs
    if not string.find(package.cpath, yawe_config.dll_path) then
        package.cpath = package.cpath .. [[;]] .. yawe_config.dll_path .. [[?.dll;]]
    else
        writeLog(log.INFO, "dll path already in cpath.")
    end

    yawe_config = {}
    _G.yawe_config = yawe_config

    local file, err = io.open(lfs.writedir() .. [[Config\yawe-config.lua]], "r")
    if file then
        local f = assert(loadstring(file:read("*all")))
        setfenv(f, yawe_config)
        f()
        writeLog(log.INFO, "`Config/yawe-config.lua` successfully read")
    else
        writeLog(log.INFO, "`Config/yawe-config.lua` not found (" .. tostring(err) .. ")")
    end
    yawe_config.write_dir = lfs.writedir()
    writeLog(log.INFO, "Yawe config follows: ")
    for k, v in pairs(yawe_config) do
        writeLog(log.INFO, k .. " = " .. tostring(v))
    end
    writeLog(log.INFO, "End of Yawe config")

    local yawe_lib = require("yawe_shim")
    if yawe_lib then
        writeLog(log.INFO, "Loaded yawe library from hook")
        yawe_lib.start(yawe_config)
        writeLog(log.INFO, "Started yawe library from hook.")
        YAWE['lib'] = yawe_lib
    else
        writeLog(log.ERROR, "Failed to load yawe library from hook")
    end
end

do
    function yaweCallbacks.onMissionLoadEnd()
        local status, err = pcall(onMissionLoadEnd)
        if not status then
            writeLog(log.INFO, "error starting library: " .. tostring(err))
        end
    end

    function yaweCallbacks.onSimulationStop()
        YAWE.lib.stop()
        YAWE.lib = nil
        YAWE = {}
        package.loaded['dcs_yawe'] = nil
    end

    function yaweCallbacks.onSimulationFrame()
        if not YAWE.lib then
            return
        end
        YAWE.lib.on_frame_begin()
    end

    function yaweCallbacks.onPlayerConnect(id)
    end

    function yaweCallbacks.onPlayerDisconnect(id, err_code)
    end

    DCS.setUserCallbacks(yaweCallbacks)
    writeLog(log.INFO, "Set up Yawe hook callbacks.")
end
