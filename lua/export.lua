do
    local function initExport()
        local YAWE = {}
        local function writeLog(level, message)
            log.write("[yawe-export]", level, message)
        end

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
            writeLog(log.INFO, "Loaded yawe library from export")
            YAWE['lib'] = yawe_lib
        else
            writeLog(log.ERROR, "Failed to load yawe library from hook")
            return
        end

        local _prevLuaExportAfterNextFrame = LuaExportAfterNextFrame
        -- local _prevLuaExportBeforeNextFrame = LuaExportBeforeNextFrame

        LuaExportAfterNextFrame = function()
            yawe_lib.on_frame_export()
            if _prevLuaExportAfterNextFrame then
                _prevLuaExportAfterNextFrame()
            end
        end

    end
    initExport()
end
