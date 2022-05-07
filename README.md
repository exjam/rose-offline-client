# rose-offline-client
An open source client for ROSE Online, currently only compatible with irose_129_129en. For a matching open source server see [rose-offline](https://github.com/exjam/rose-offline/)

# Running
## Required arguments:
- `--data-idx=<path/to/data.idx>` Path to irose 129en data.idx

## Game Mode
Use `--game` to run in game mode, this is where we act as a full client which requires an irose 129en compatible server.
- `--ip` Server IP for login server
- `--port` Server port for login server (defaults to 29000)

You can also use `--auto-login` for automatic login in game mode.
- `--username=<username>` Username for auto login
- `--password=<password>` Password for auto login
- `--server-id=<N>` Server ID for auto login (defaults to 0)
- `--channel-id=<N>` Channel ID for auto login (defaults to 0)
- `--character-name=<name>` Character name for auto login (optional, auto login can be username/password only)

## Model Viewer Mode
Use `--model-viewer` to run in model viewer mode, which allows you to view character and NPC models.

## Zone Viewer Mode
Use `--zone=<N>` to run in zone viewer mode, this allows you to view zones.

# Screenshots

<img alt="Game Mode"  src="https://user-images.githubusercontent.com/1302758/167260422-2cb29850-a049-4271-9e82-f45552c7e939.jpg">

<img alt="Model Viewer" src="https://user-images.githubusercontent.com/1302758/159884786-772d7b53-a58e-4e16-a5c9-c8ab52536afa.jpg">

<img alt="Zone Viewer - City of Junon Polis" src="https://user-images.githubusercontent.com/1302758/156855913-942e122a-c847-464b-a4be-5c41057f9265.jpg">
