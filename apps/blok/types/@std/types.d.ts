import {NumericConstraint} from "./conditions";


export interface OntologyType {
    version: number;
    archived: boolean;
}


export interface DataType extends PropertyTypeVariant {
    version: number;
}

interface PropertyTypeVariant {
    __brand: 'PropertyTypeVariant';
}

export interface Object<T extends PropertyType> extends PropertyTypeVariant {
    __type: T;
}

type ArrayItem = Object<PropertyType> | Array<ArrayItem>;

export interface Array<T extends ArrayItem> extends PropertyTypeVariant {
    __type: T;
}

export interface Ref<T extends OntologyType> extends PropertyTypeVariant {
    __type: T;
}

export interface PropertyType extends OntologyType {
    oneOf: PropertyTypeVariant
}

interface LinkProperties {
    length: NumericConstraint
}

export interface Link<T extends EntityType, U extends LinkProperties> {
    to: T
    properties: U
}

export interface EntityType extends OntologyType {
    version: number;

    properties: PropertyType | Versioned<PropertyType, PropertyType>
    links: Link<EntityType, LinkProperties>
}
