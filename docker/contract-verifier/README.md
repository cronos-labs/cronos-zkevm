# Contract verifier docker image

## Building the contract verifier docker image

- execute `zk` in the project root directory.
- simply run `zk docker build contract-verifier` in the project root directory.

## Testing the contract verifier docker image

- make sure the docker env setup for the contract verifier docker image in `docker-compose.yml`.
- update the `infrastructure/zk/src/up.ts` with

```text
function createVolumes() {
    fs.mkdirSync(`${process.env.ZKSYNC_HOME}/volumes/geth`, { recursive: true });
    fs.mkdirSync(`${process.env.ZKSYNC_HOME}/volumes/postgres`, { recursive: true });
    fs.mkdirSync(`${process.env.ZKSYNC_HOME}/volumes/contract-verifier`, { recursive: true }); // new add code
}

export async function up() {
    createVolumes();
    //await utils.spawn('docker-compose up -d geth postgres');  // comment out this line
    await utils.spawn('docker-compose up -d geth postgres contract-verifier'); // new add code
}
```

- execute `zk` to update the zk program.
- run `zk init` or `zk reinit` for setup the docker image in the zksync-era container.
- stop the contract verifier before run `zk server` and then restart the contract verifier.
