/** A deliberately unsafe cell: completion/validate fixtures for the
 * unsafe-set classification. */

// eslint-disable-next-line -- fixture: the any is the point
export const anyThing: any = 1;

export function useAny(): number {
  return anyThing + 1;
}
