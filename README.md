# Amax ADS
## Restoration of in-game ads functionality for Amax EMU project.

This dll with a power of 2 detours populating empty and lifeless billboards on all tracks in game. Images are coming from S3 storage, and kept in [amax-ads-files](https://github.com/Amax-Emu/amax-ads-files) repo.

![In game screenshot](https://amax-ads.fra1.cdn.digitaloceanspaces.com/blur_ads.png)


### TODO
- [ ] Clean up `hook_enter_zone_post_load`.
- [ ] Get rid of `d3dx9_42.dll!D3DXCreateTextureFromFileInMemoryEx(..)`.
- [ ] More robust `G_CACHE` init, currently there is a lot of fragile stuff happenin in that LazyLock, don't like it.
- [ ] Simplify download.
- [ ] Some docs, especially on Caches and game structs!
