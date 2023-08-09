import { Property, Versioned } from '@std/types';
import { Text } from '@std/data';

/**
 * Secondary E-Mail Address
 *
 * Secondary E-Mail Address of an entity.
 *
 * @id secondary-email
 */
interface V1 extends Property {
    version: 1;

    oneOf: Text;
}


export type SecondaryEMail = Versioned<Property, V1>;
