// Polyfill for TextDecoder
if (typeof TextDecoder === 'undefined') {
    globalThis.TextDecoder = class TextDecoder {
        constructor(encoding = 'utf-8') {
            // Normalize and validate the encoding parameter
            const normalizedEncoding = encoding.toLowerCase().replace('-', '');
            if (normalizedEncoding !== 'utf8') {
                throw new RangeError(`Encoding '${encoding}' not supported. Only UTF-8 is supported.`);
            }
            this.encoding = 'utf-8';
        }
        
        /**
         * Decodes a buffer of UTF-8 bytes into a string
         * @param {ArrayBuffer|ArrayBufferView} bytes - The bytes to decode
         * @returns {string} The decoded string
         */
        decode(bytes) {
            if (!bytes) return '';
            
            // Convert input to Uint8Array for consistent processing
            const uint8Array = bytes instanceof Uint8Array ? bytes : new Uint8Array(bytes);
            let result = '';
            
            for (let i = 0; i < uint8Array.length; i++) {
                const byte = uint8Array[i];
                
                // 1-byte sequence (ASCII)
                if (byte < 128) {
                    result += String.fromCharCode(byte);
                } 
                // 2-byte sequence
                else if (byte < 224) {
                    // Check if there are enough bytes remaining
                    if (i + 1 >= uint8Array.length) {
                        result += '\uFFFD'; // Replacement character for truncated sequence
                        break;
                    }
                    const byte2 = uint8Array[++i];
                    // Validate continuation byte (10xxxxxx)
                    if ((byte2 & 0xC0) !== 0x80) {
                        result += '\uFFFD';
                        continue;
                    }
                    result += String.fromCharCode(((byte & 31) << 6) | (byte2 & 63));
                } 
                // 3-byte sequence
                else if (byte < 240) {
                    if (i + 2 >= uint8Array.length) {
                        result += '\uFFFD';
                        break;
                    }
                    const byte2 = uint8Array[++i];
                    const byte3 = uint8Array[++i];
                    // Validate continuation bytes
                    if ((byte2 & 0xC0) !== 0x80 || (byte3 & 0xC0) !== 0x80) {
                        result += '\uFFFD';
                        continue;
                    }
                    const codePoint = ((byte & 15) << 12) | ((byte2 & 63) << 6) | (byte3 & 63);
                    
                    // Check for overlong encoding
                    if (codePoint < 0x800) {
                        result += '\uFFFD';
                    } else {
                        result += String.fromCharCode(codePoint);
                    }
                } 
                // 4-byte sequence
                else if (byte < 248) {
                    if (i + 3 >= uint8Array.length) {
                        result += '\uFFFD';
                        break;
                    }
                    const byte2 = uint8Array[++i];
                    const byte3 = uint8Array[++i];
                    const byte4 = uint8Array[++i];
                    // Validate continuation bytes
                    if ((byte2 & 0xC0) !== 0x80 || (byte3 & 0xC0) !== 0x80 || (byte4 & 0xC0) !== 0x80) {
                        result += '\uFFFD';
                        continue;
                    }
                    const codePoint = ((byte & 7) << 18) | ((byte2 & 63) << 12) | ((byte3 & 63) << 6) | (byte4 & 63);
                    
                    // Check if code point is valid Unicode and not overlong
                    if (codePoint <= 0x10FFFF && codePoint >= 0x10000) {
                        result += String.fromCodePoint(codePoint);
                    } else {
                        result += '\uFFFD'; // Invalid Unicode code point
                    }
                } 
                // Invalid UTF-8 starting byte
                else {
                    result += '\uFFFD';
                }
            }
            return result;
        }
    };
}

// Polyfill for TextEncoder
if (typeof TextEncoder === 'undefined') {
    globalThis.TextEncoder = class TextEncoder {
        constructor() {
            this.encoding = 'utf-8';
        }
        
        /**
         * Encodes a string into UTF-8 bytes
         * @param {string} str - The string to encode
         * @returns {Uint8Array} The encoded UTF-8 bytes
         */
        encode(str) {
            const result = [];
            
            for (let i = 0; i < str.length; i++) {
                let codePoint = str.charCodeAt(i);
                
                // Handle UTF-16 surrogate pairs
                if (codePoint >= 0xD800 && codePoint <= 0xDBFF) {
                    // High surrogate
                    if (i + 1 < str.length) {
                        const nextCode = str.charCodeAt(i + 1);
                        if (nextCode >= 0xDC00 && nextCode <= 0xDFFF) {
                            // Low surrogate - combine to form a single code point
                            codePoint = ((codePoint - 0xD800) << 10) + (nextCode - 0xDC00) + 0x10000;
                            i++;
                        }
                    }
                    // If there's no valid low surrogate, treat high surrogate as invalid
                    // It will be encoded as replacement character below
                }
                
                // Encode the code point to UTF-8
                if (codePoint < 0x80) {
                    // 1-byte sequence (ASCII)
                    result.push(codePoint);
                } else if (codePoint < 0x800) {
                    // 2-byte sequence
                    result.push(0xC0 | (codePoint >> 6));
                    result.push(0x80 | (codePoint & 0x3F));
                } else if (codePoint < 0x10000) {
                    // 3-byte sequence
                    result.push(0xE0 | (codePoint >> 12));
                    result.push(0x80 | ((codePoint >> 6) & 0x3F));
                    result.push(0x80 | (codePoint & 0x3F));
                } else if (codePoint <= 0x10FFFF) {
                    // 4-byte sequence
                    result.push(0xF0 | (codePoint >> 18));
                    result.push(0x80 | ((codePoint >> 12) & 0x3F));
                    result.push(0x80 | ((codePoint >> 6) & 0x3F));
                    result.push(0x80 | (codePoint & 0x3F));
                } else {
                    // Invalid Unicode code point - use replacement character (U+FFFD)
                    result.push(0xEF, 0xBF, 0xBD);
                }
            }
            
            return new Uint8Array(result);
        }
    };
}

