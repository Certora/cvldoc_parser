//// # NatSpec test file for the 'definition' CVL command.


/**
 * @title harness_isListed  - is listed harness.
 * @param a sender's address
 * @param i the value that is tested.
 * @return true if the iterm is listed. false, otherwise.
 */
definition harness_isListed(address a, uint i) returns bool = 0 <= i && i < shadowLenArray() && shadowArray(i) == a ;


/**
 * @title MAX_UINT160  - maximum integer.
 * @notice - returns the value of maximum integer in 160 bits.
 * @return the maximum value.
 */
definition MAX_UINT160() returns uint = 1461501637330902918203684832716283019655932542975;