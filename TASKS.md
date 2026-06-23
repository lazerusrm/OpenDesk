# Tasks

## Immediate Next Steps

- [ ] Create Git repository and remote.
- [ ] Decide first implementation stack: Go server-rendered UI vs FastAPI + React.
- [ ] Inventory current RustDesk LXC configuration.
- [ ] Export current RustDesk public key and document client config.
- [ ] Define required day-one OS targets.
- [ ] Decide whether the first deployment runs in the existing LXC or a new LXC.
- [x] Clone upstream RustDesk client/server locally for reference.
- [x] Document upstream hooks relevant to OpenDesk.
- [x] Create Pro feature parity map.

## First Build Slice

- [ ] Scaffold backend service.
- [ ] Add SQLite migrations.
- [ ] Add admin login.
- [ ] Add device CRUD.
- [ ] Add server config settings page.
- [ ] Add Windows install/config script generator.
- [ ] Add Linux install/config script generator.
- [ ] Add endpoint enrollment token model.
- [ ] Add endpoint registration endpoint.

## Questions For Owner

- Which RustDesk Pro features are used every week today?
- Are passwords stored in the RustDesk Pro address book today?
- Which endpoints matter first: Windows desktops, Linux desktops, servers, macOS, mobile?
- Is there already a reverse proxy/auth stack for `example.com` services?
- Do we want `rd-admin.example.com` as the dashboard hostname?
