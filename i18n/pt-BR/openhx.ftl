waiting-title = Aguardando dispositivo...
waiting-subtitle = Conecte seu HX Stomp ou HX Stomp XL via USB.
connected-header = Conectado: { $device_name }
error-title = Erro de Comunicação
error-unknown = Erro desconhecido

cli-list-presets-about = Lista todos os presets de um dispositivo Line 6 conectado.
cli-list-presets-long = Verifica o barramento USB automaticamente se nenhum dispositivo for especificado.
cli-device-help = Alveja um dispositivo específico em vez de detectar automaticamente.
cli-connecting-to = Conectando a { $device_name } …
cli-probing-usb = Nenhum dispositivo especificado — verificando barramento USB por qualquer dispositivo Line 6 suportado...
cli-connected-to = Conectado a: { $profile }
cli-total-presets = Total: { $count } preset(s) lido(s).

usb-detected = Detectado: { $device }
usb-device-unresponsive = Dispositivo '{ $device }' não respondeu após { $attempts } tentativas.
usb-kernel-detach-failed = Falha ao desanexar o kernel: { $error }
usb-stream-offset-overflow = Estouro de offset do fluxo no payload USB.
usb-retry-attempt = [{ $device }] Tentativa { $current }/{ $total } falhou. Tentando novamente em { $wait_ms } ms...

msgpack-root-not-array = O valor raiz do MessagePack não é um array.
msgpack-preset-not-map = O item do preset não é um mapa.
msgpack-preset-map-empty = O mapa do item do preset está vazio.
msgpack-preset-index-not-int = O índice do preset não é um número inteiro.
msgpack-preset-inner-not-map = Preset { $index }: o mapa de propriedades não é um mapa.
msgpack-preset-name-not-found = Preset { $index }: chave de nome não encontrada ou inválida.
