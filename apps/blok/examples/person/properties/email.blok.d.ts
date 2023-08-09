import { Property, Versioned } from '@std/types';
import { Text } from '@std/data';

/**
 * E-Mail
 *
 * E-Mail Address of an entity.
 *
 * @id email
 */
interface V1 extends Property {
    version: 1;

    oneOf: Text;
}

export type EMail = Versioned<Property, V1>;
