waiting-title = Waiting for device...
waiting-subtitle = Connect your HX Stomp or HX Stomp XL via USB.
connected-header = Connected: { $device_name }
error-title = Communication Error
error-unknown = Unknown error

cli-list-presets-about = Lists all presets from a connected Line 6 device.
cli-list-presets-long = Probes the USB bus automatically if no device is specified.
cli-preset-category-about = Manage, list, and modify device presets.
cli-device-help = Target a specific device instead of auto-detecting.
cli-connecting-to = Connecting to { $device_name } …
cli-probing-usb = No device specified — probing USB bus for any supported Line 6 device...
cli-connected-to = Connected to: { $profile }
cli-total-presets = Total: { $count } preset(s) read.

cli-select-preset-about = Selects a preset on the connected device.
cli-select-preset-long =
    Switches the active preset to the given 0-indexed slot.
    The index matches what the device display shows: 0 = "00", 1 = "01", etc.
    Note: HX Edit labels are 1-indexed — subtract 1 when converting.
cli-select-preset-index-help = Preset index to activate (0-indexed, e.g. 0 = "00" on the device).
cli-selecting-preset = Selecting preset { $index } …
cli-preset-selected = ✓ Preset { $index } activated: { $name }

usb-detected = Detected: { $device }
usb-device-unresponsive = Device '{ $device }' unresponsive after { $attempts } attempts.
usb-kernel-detach-failed = Kernel detach failed: { $error }
usb-stream-offset-overflow = Stream offset overflow in USB payload.
usb-retry-attempt = [{ $device }] Attempt { $current }/{ $total } failed. Retrying in { $wait_ms } ms...

msgpack-root-not-array = Root MessagePack value is not an array.
msgpack-preset-not-map = Preset item is not a map.
msgpack-preset-map-empty = Preset item map is empty.
msgpack-preset-index-not-int = Preset index is not an integer.
msgpack-preset-inner-not-map = Preset { $index }: property map is not a map.
msgpack-preset-name-not-found = Preset { $index }: name key not found or invalid.
