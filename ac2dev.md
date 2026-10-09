# Requests for ac2 developers

## Protocol 34 dependency refresh

The remote viewer needs rebuilding against protocol 34. The sibling checkout at
`/home/mui/src/ac2` was last inspected at `5e267e1` (protocol 33); its cached
`origin/main` was `064ed8b` (protocol 34). Protocol 34 adds saved impulse responses
to transfer traces, so refreshing may also require adapting viewer trace fixtures
and calls into the shared scene code.

Please provide a protocol-34 dependency checkout/revision, or update the sibling
checkout through the ac2 development process. The remote-viewer agent must leave
`~/src/ac2` untouched. Once the compatible sources are available, the viewer can
be adapted, tested, and rebuilt here. The latest rebuilt APK still uses protocol 33.
