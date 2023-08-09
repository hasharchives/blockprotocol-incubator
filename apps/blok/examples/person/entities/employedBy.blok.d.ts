import {Entity, Versioned} from '@std/types';

/**
 * Employed By
 *
 * Being paid to work for this entity.
 *
 * @id employed-by
 */
interface V1 extends Entity {
    version: 1
}

export type EmployedBy = Versioned<Entity, V1>;
