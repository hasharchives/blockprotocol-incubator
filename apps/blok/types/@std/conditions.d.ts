// This is just for visualization, not actually using them
type Add1<T> = T;
type Sub1<T> = T;

export interface NumericConstraint {
    // Inclusive minimum
    min?: number;
    // Inclusive maximum
    max?: number;
}

export interface GreaterThan<T extends number> {
    min: Sub1<T>;
}

export interface LessThan<T extends number> {
    max: Add1<T>;
}

export interface EqualTo<T extends number> {
    min: T;
    max: T;
}

export interface GreaterThanOrEqualTo<T extends number> {
    min: T;
}

export interface LessThanOrEqualTo<T extends number> {
    max: T;
}
