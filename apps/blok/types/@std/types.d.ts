import {NumericConstraint} from "./conditions";

export interface Versioned<T, V extends T> {
    versions: V;
}

export interface Data {
    version: number;
}

export interface Property {
    version: number;

    oneOf: Data | Versioned<Data, Data>
}

interface LinkProperties {
    length: NumericConstraint
}

export interface Link<T extends Entity, U extends LinkProperties> {
    to: T
    properties: U
}

export interface Entity {
    version: number;

    properties: Property | Versioned<Property, Property>
    links: Link<Entity, LinkProperties>
}
