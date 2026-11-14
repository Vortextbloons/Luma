[void][Windows.Networking.Connectivity.NetworkInformation,Windows.Networking.Connectivity,ContentType=WindowsRuntime]
$profiles = [Windows.Networking.Connectivity.NetworkInformation]::GetConnectionProfiles()
foreach ($p in $profiles) {
    $lvl = $p.GetNetworkConnectivityLevel()
    $kind = $null
    try { $kind = $p.NetworkAdapter().IanaInterfaceType() } catch {}
    "{0} | connectivity={1} | ianaType={2}" -f $p.ProfileName, $lvl, $kind
}
