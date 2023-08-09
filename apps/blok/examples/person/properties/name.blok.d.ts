import { Property, Versioned } from '@std/types';
import { Text } from '@std/data';

/**
 * Name
 *
 * Name of an entity.
 *
 * @id name
 */
interface V1 extends Property {
    version: 1;

    oneOf: Text;
}

export type Name = Versioned<Property, V1>;
