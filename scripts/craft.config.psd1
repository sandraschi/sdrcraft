# craft.config.psd1 - per-repo settings for start.ps1 and scripts\release.ps1.
# Floor = $true: start.ps1 runs scripts\craft-floor.ps1 and stays closed when it fails.
@{
    Name        = 'sdrcraft'
    Display     = 'SDRCraft'
    AppPackage  = 'sdrcraft-app'
    AppBin      = 'sdrcraft'
    AppSmoke    = @('--smoke')
    CliPackage  = 'sdrcraft-mcp'
    CliBin      = 'sdrcraft-mcp'
    CliSmoke    = @('status')
    McpPackage  = 'sdrcraft-mcp'
    McpBin      = 'sdrcraft-mcp'
    McpArgs     = @('mcp-stdio')
    McpSmoke    = @()
    Installer   = $true
    Mcpb        = $false
    Floor       = $true
}
