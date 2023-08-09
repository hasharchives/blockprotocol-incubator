import {Entity, Versioned, Link} from '@std/types';
import {GreaterThanEqual} from '@std/conditions';
import {EmployedBy} from "./employedBy.blok";
import {EMail} from "../properties/email.blok";
import {Name} from "../properties/name.blok";


/**
 * Person
 *
 * An extremely simplified representation of a person or human being.
 *
 * @id person
 */
interface V1 extends Entity {
    version: 1;

    properties: EMail & Name;
    links: Link<EmployedBy, { length: GreaterThanEqual<0>; }>;
}

// To constrain a property on array length use: Array<Name, { length: GreaterThanEqual<1> }> instead of Name[]

export type Person = Versioned<Entity, V1>;
