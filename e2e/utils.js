export const parseError = (chain, error) => {
  const decoded = chain.registry.findMetaError(error.asModule);
  const { docs, name, section } = decoded;

  return `${section}.${name}: ${docs.join(' ')}`;
};

export const submit = (chain, extrinsic, keypair) => {
  return new Promise((resolve, reject) => {
    try {
      extrinsic.signAndSend(
        keypair,
        { nonce: -1 },
        ({ events, status, dispatchError }) => {
          if (dispatchError) {
            reject(new Error(parseError(chain, dispatchError)));
          } else if (status.isInBlock) {
            // eslint-disable-next-line no-restricted-syntax
            for (const { data, method, section } of events) {
              if (section === 'magic' && method === 'Codo' && data[0].isError) {
                const error = data[0].asError;
                reject(new Error(parseError(chain, error)));
                return;
              }
            }

            resolve({
              tx: extrinsic.hash.toHex(),
              block: status.asInBlock.toHex(),
            });
          }
        }
      );
    } catch (e) {
      reject(e);
    }
  });
};
