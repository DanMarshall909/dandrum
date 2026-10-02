// Read-only requests are coalesced. Newer live state closes renderer admission
// until its matching document arrives; late replies cannot restore old metadata.
export function createPreparedParameterDocument(request, onDocument, onError) {
  let generation = -1;
  let documentGeneration = -1;
  let pending = null;
  let closed = false;

  const refresh = () => {
    if (closed) return Promise.resolve(false);
    if (pending) return pending;
    const requestedGeneration = generation;
    pending = Promise.resolve().then(request).then(next => {
      if (closed || !Number.isInteger(next?.generation)
          || next.generation < generation || next.generation < documentGeneration) return false;
      documentGeneration = next.generation;
      onDocument(next);
      return true;
    }).catch(reason => {
      if (!closed) onError(reason);
      return false;
    }).finally(() => {
      pending = null;
      if (!closed && generation > requestedGeneration && documentGeneration < generation)
        void refresh();
    });
    return pending;
  };
  return {
    acceptGeneration(next) {
      if (closed || !Number.isInteger(next) || next < 0) return Promise.resolve(false);
      generation = Math.max(generation, next);
      return documentGeneration >= generation ? Promise.resolve(true) : refresh();
    },
    refresh,
    close() { closed = true; },
  };
}
